use crate::error::ContractError;
use crate::msg::{
    CW20EntitlementResponse, CW20Response, DenomResponse, ExecuteMsg, InstantiateMsg, MigrateMsg,
    NativeEntitlementResponse, QueryMsg, TotalPowerResponse, VotingContractResponse,
};
use crate::state::{
    CW20_BALANCES, CW20_CLAIMS, DISTRIBUTION_HEIGHT, FUNDING_PERIOD_EXPIRATION, NATIVE_BALANCES,
    NATIVE_CLAIMS, TOTAL_POWER, VOTING_CONTRACT,
};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Addr, BankMsg, Binary, Coin, Decimal, Deps, DepsMut, Env, Fraction,
    MessageInfo, Order, Response, StdError, StdResult, Uint128, WasmMsg,
};
use cw2::set_contract_version;
use cw_paginate_storage::paginate_map;

use dao_interface::voting;

const CONTRACT_NAME: &str = "crates.io:cw-fund-distributor";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

type NativeClaimEntry = Result<((Addr, String), Uint128), StdError>;
type Cw20ClaimEntry = Result<((Addr, Addr), Uint128), StdError>;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // store the height
    DISTRIBUTION_HEIGHT.save(deps.storage, &msg.distribution_height)?;

    // get the funding expiration and store it
    let funding_expiration_height = msg.funding_period.after(&env.block);
    FUNDING_PERIOD_EXPIRATION.save(deps.storage, &funding_expiration_height)?;

    // validate the contract and save it
    let voting_contract = deps.api.addr_validate(&msg.voting_contract)?;
    VOTING_CONTRACT.save(deps.storage, &voting_contract)?;

    let total_power: voting::TotalPowerAtHeightResponse = deps.querier.query_wasm_smart(
        voting_contract.clone(),
        &voting::Query::TotalPowerAtHeight {
            height: Some(env.block.height),
        },
    )?;
    // validate the total power and store it
    if total_power.power.is_zero() {
        return Err(ContractError::ZeroVotingPower {});
    }
    TOTAL_POWER.save(deps.storage, &total_power.power)?;

    Ok(Response::default()
        .add_attribute("distribution_height", env.block.height.to_string())
        .add_attribute("voting_contract", voting_contract)
        .add_attribute("total_power", total_power.power))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Receive(cw20::Cw20ReceiveMsg {
            sender: _,
            amount,
            msg: _,
        }) => execute_fund_cw20(deps, env, info.sender, amount),
        ExecuteMsg::FundNative {} => execute_fund_native(deps, env, info),
        ExecuteMsg::ClaimCW20 { tokens } => execute_claim_cw20s(deps, env, info.sender, tokens),
        ExecuteMsg::ClaimNatives { denoms } => {
            execute_claim_natives(deps, env, info.sender, denoms)
        }
        ExecuteMsg::ClaimAll {} => execute_claim_all(deps, env, info.sender),
    }
}

pub fn execute_fund_cw20(
    deps: DepsMut,
    env: Env,
    token: Addr,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let funding_deadline = FUNDING_PERIOD_EXPIRATION.load(deps.storage)?;
    // if current block indicates claiming period, return an error
    if funding_deadline.is_expired(&env.block) {
        return Err(ContractError::FundDuringClaimingPeriod {});
    }

    if amount > Uint128::zero() {
        CW20_BALANCES.update(
            deps.storage,
            token.clone(),
            |current_balance| -> Result<_, ContractError> {
                match current_balance {
                    // add the funding amount to current balance
                    Some(old_amount) => Ok(old_amount.checked_add(amount)?),
                    // with no existing balance, set it to the funding amount
                    None => Ok(amount),
                }
            },
        )?;
    }

    Ok(Response::default()
        .add_attribute("method", "fund_cw20")
        .add_attribute("token", token)
        .add_attribute("amount", amount))
}

pub fn execute_fund_native(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let funding_deadline = FUNDING_PERIOD_EXPIRATION.load(deps.storage)?;
    // if current block indicates claiming period, return an error
    if funding_deadline.is_expired(&env.block) {
        return Err(ContractError::FundDuringClaimingPeriod {});
    }

    // collect a list of successful funding kv pairs
    let mut attributes: Vec<(String, String)> = Vec::new();
    for coin in info.funds {
        if coin.amount > Uint128::zero() {
            NATIVE_BALANCES.update(
                deps.storage,
                coin.denom.clone(),
                |current_balance| -> Result<_, ContractError> {
                    let new_amount = match current_balance {
                        // add the funding amount to current balance
                        Some(current_balance) => coin.amount.checked_add(current_balance)?,
                        // with no existing balance, set it to the funding amount
                        None => coin.amount,
                    };
                    attributes.push((coin.denom, new_amount.to_string()));
                    Ok(new_amount)
                },
            )?;
        }
    }

    Ok(Response::default()
        .add_attribute("method", "fund_native")
        .add_attributes(attributes))
}

fn get_entitlement(
    distributor_funds: Uint128,
    relative_share: Decimal,
    previous_claim: Uint128,
) -> Result<Uint128, ContractError> {
    let total_share =
        distributor_funds.multiply_ratio(relative_share.numerator(), relative_share.denominator());
    match total_share.checked_sub(previous_claim) {
        Ok(entitlement) => Ok(entitlement),
        Err(e) => Err(ContractError::OverflowErr(e)),
    }
}

fn get_relative_share(deps: &Deps, sender: Addr) -> Result<Decimal, StdError> {
    let voting_contract = VOTING_CONTRACT.load(deps.storage)?;
    let dist_height = DISTRIBUTION_HEIGHT.load(deps.storage)?;
    let total_power = TOTAL_POWER.load(deps.storage)?;

    // find the voting power of sender at distributor instantiation
    let voting_power: voting::VotingPowerAtHeightResponse = deps.querier.query_wasm_smart(
        voting_contract,
        &voting::Query::VotingPowerAtHeight {
            address: sender.to_string(),
            height: Some(dist_height),
        },
    )?;
    // return senders share
    Ok(Decimal::from_ratio(voting_power.power, total_power))
}

pub fn execute_claim_cw20s(
    deps: DepsMut,
    env: Env,
    sender: Addr,
    tokens: Vec<String>,
) -> Result<Response, ContractError> {
    let funding_deadline = FUNDING_PERIOD_EXPIRATION.load(deps.storage)?;
    // if current block indicates funding period, return an error
    if !funding_deadline.is_expired(&env.block) {
        return Err(ContractError::ClaimDuringFundingPeriod {});
    }
    if tokens.is_empty() {
        return Err(ContractError::EmptyClaim {});
    }

    let relative_share = get_relative_share(&deps.as_ref(), sender.clone())?;
    let messages = get_cw20_claim_wasm_messages(tokens, deps, sender.clone(), relative_share)?;

    Ok(Response::default()
        .add_attribute("method", "claim_cw20s")
        .add_attribute("sender", sender)
        .add_messages(messages))
}

/// Looks at the CW20_BALANCES map entries and returns a vector of WasmMsg::Execute
/// messages that entail the amount that the user is entitled to.
/// Updates the CW20_CLAIMS entries accordingly.
fn get_cw20_claim_wasm_messages(
    tokens: Vec<String>,
    deps: DepsMut,
    sender: Addr,
    relative_share: Decimal,
) -> Result<Vec<WasmMsg>, ContractError> {
    let mut messages: Vec<WasmMsg> = vec![];
    for addr in tokens {
        // get the balance of distributor at instantiation
        let bal = CW20_BALANCES.load(deps.storage, Addr::unchecked(addr.clone()))?;

        // check for any previous claims
        let previous_claim = CW20_CLAIMS
            .may_load(
                deps.storage,
                (sender.clone(), Addr::unchecked(addr.clone())),
            )?
            .unwrap_or_default();

        // get % share of sender and subtract any previous claims
        let entitlement = get_entitlement(bal, relative_share, previous_claim)?;
        if !entitlement.is_zero() {
            // reflect the new total claim amount
            CW20_CLAIMS.update(
                deps.storage,
                (sender.clone(), Addr::unchecked(addr.clone())),
                |claim| match claim {
                    Some(previous_claim) => previous_claim
                        .checked_add(entitlement)
                        .map_err(ContractError::OverflowErr),
                    None => Ok(entitlement),
                },
            )?;

            messages.push(WasmMsg::Execute {
                contract_addr: addr,
                msg: to_json_binary(&cw20::Cw20ExecuteMsg::Transfer {
                    recipient: sender.to_string(),
                    amount: entitlement,
                })?,
                funds: vec![],
            });
        }
    }

    Ok(messages)
}

pub fn execute_claim_natives(
    deps: DepsMut,
    env: Env,
    sender: Addr,
    denoms: Vec<String>,
) -> Result<Response, ContractError> {
    let funding_deadline = FUNDING_PERIOD_EXPIRATION.load(deps.storage)?;
    // if current block indicates funding period, return an error
    if !funding_deadline.is_expired(&env.block) {
        return Err(ContractError::ClaimDuringFundingPeriod {});
    }
    if denoms.is_empty() {
        return Err(ContractError::EmptyClaim {});
    }

    // find the relative share of the distributor pool for the user
    // and determine the native claim transfer amounts with it
    let relative_share = get_relative_share(&deps.as_ref(), sender.clone())?;
    let messages = get_native_claim_bank_messages(denoms, deps, sender.clone(), relative_share)?;

    Ok(Response::default()
        .add_attribute("method", "claim_natives")
        .add_attribute("sender", sender)
        .add_messages(messages))
}

/// Looks at the NATIVE_BALANCES map entries and returns a vector of
/// BankMsg::Send messages that entail the amount that the user is
/// entitled to. Updates the NATIVE_CLAIMS entries accordingly.
fn get_native_claim_bank_messages(
    denoms: Vec<String>,
    deps: DepsMut,
    sender: Addr,
    relative_share: Decimal,
) -> Result<Vec<BankMsg>, ContractError> {
    let mut messages: Vec<BankMsg> = vec![];

    for addr in denoms {
        // get the balance of distributor at instantiation
        let bal = NATIVE_BALANCES.load(deps.storage, addr.clone())?;

        // check for any previous claims
        let previous_claim = NATIVE_CLAIMS
            .may_load(deps.storage, (sender.clone(), addr.clone()))?
            .unwrap_or_default();

        // get % share of sender and subtract any previous claims
        let entitlement = get_entitlement(bal, relative_share, previous_claim)?;
        if !entitlement.is_zero() {
            // reflect the new total claim amount
            NATIVE_CLAIMS.update(
                deps.storage,
                (sender.clone(), addr.clone()),
                |claim| match claim {
                    Some(previous_claim) => previous_claim
                        .checked_add(entitlement)
                        .map_err(ContractError::OverflowErr),
                    None => Ok(entitlement),
                },
            )?;

            // collect the transfer messages
            messages.push(BankMsg::Send {
                to_address: sender.to_string(),
                amount: vec![Coin {
                    denom: addr,
                    amount: entitlement,
                }],
            });
        }
    }
    Ok(messages)
}

pub fn execute_claim_all(
    mut deps: DepsMut,
    env: Env,
    sender: Addr,
) -> Result<Response, ContractError> {
    let funding_deadline = FUNDING_PERIOD_EXPIRATION.load(deps.storage)?;
    // claims cannot happen during funding period
    if !funding_deadline.is_expired(&env.block) {
        return Err(ContractError::ClaimDuringFundingPeriod {});
    }

    // get the lists of tokens in distributor pool
    let cw20s: Vec<Result<Addr, _>> = CW20_BALANCES
        .keys(deps.storage, None, None, Order::Ascending)
        .collect();
    let mut cw20_addresses: Vec<String> = vec![];
    for entry in cw20s {
        cw20_addresses.push(entry?.to_string());
    }

    let native_denoms: Vec<Result<String, _>> = NATIVE_BALANCES
        .keys(deps.storage, None, None, Order::Ascending)
        .collect();
    let mut denoms = vec![];
    for denom in native_denoms {
        denoms.push(denom?);
    }

    let relative_share = get_relative_share(&deps.as_ref(), sender.clone())?;

    // get the claim messages
    let cw20_claim_msgs = get_cw20_claim_wasm_messages(
        cw20_addresses,
        deps.branch(),
        sender.clone(),
        relative_share,
    )?;
    let native_claim_msgs =
        get_native_claim_bank_messages(denoms, deps.branch(), sender, relative_share)?;

    Ok(Response::default()
        .add_attribute("method", "claim_all")
        .add_messages(cw20_claim_msgs)
        .add_messages(native_claim_msgs))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::VotingContract {} => query_voting_contract(deps),
        QueryMsg::TotalPower {} => query_total_power(deps),
        QueryMsg::NativeDenoms {} => query_native_denoms(deps),
        QueryMsg::CW20Tokens {} => query_cw20_tokens(deps),
        QueryMsg::NativeEntitlement { sender, denom } => {
            query_native_entitlement(deps, sender, denom)
        }
        QueryMsg::CW20Entitlement { sender, token } => query_cw20_entitlement(deps, sender, token),
        QueryMsg::NativeEntitlements {
            sender,
            start_at,
            limit,
        } => query_native_entitlements(deps, sender, start_at, limit),
        QueryMsg::CW20Entitlements {
            sender,
            start_at,
            limit,
        } => query_cw20_entitlements(deps, sender, start_at, limit),
    }
}

pub fn query_voting_contract(deps: Deps) -> StdResult<Binary> {
    let contract = VOTING_CONTRACT.load(deps.storage)?;
    let distribution_height = DISTRIBUTION_HEIGHT.load(deps.storage)?;
    to_json_binary(&VotingContractResponse {
        contract,
        distribution_height,
    })
}

pub fn query_total_power(deps: Deps) -> StdResult<Binary> {
    let total_power: Uint128 = TOTAL_POWER.may_load(deps.storage)?.unwrap_or_default();
    to_json_binary(&TotalPowerResponse { total_power })
}

pub fn query_native_denoms(deps: Deps) -> StdResult<Binary> {
    let native_balances = NATIVE_BALANCES.range(deps.storage, None, None, Order::Ascending);

    let mut denom_responses: Vec<DenomResponse> = vec![];
    for entry in native_balances {
        let (denom, amount) = entry?;
        denom_responses.push(DenomResponse {
            contract_balance: amount,
            denom,
        });
    }

    to_json_binary(&denom_responses)
}

pub fn query_cw20_tokens(deps: Deps) -> StdResult<Binary> {
    let cw20_balances = CW20_BALANCES.range(deps.storage, None, None, Order::Ascending);

    let mut cw20_responses: Vec<CW20Response> = vec![];
    for cw20 in cw20_balances {
        let (token, amount) = cw20?;
        cw20_responses.push(CW20Response {
            contract_balance: amount,
            token: token.to_string(),
        });
    }

    to_json_binary(&cw20_responses)
}

pub fn query_native_entitlement(deps: Deps, sender: Addr, denom: String) -> StdResult<Binary> {
    let address = deps.api.addr_validate(sender.as_ref())?;
    let prev_claim = NATIVE_CLAIMS
        .may_load(deps.storage, (address, denom.clone()))?
        .unwrap_or_default();
    let total_bal = NATIVE_BALANCES
        .may_load(deps.storage, denom.clone())?
        .unwrap_or_default();
    let relative_share = get_relative_share(&deps, sender)?;

    let total_share =
        total_bal.multiply_ratio(relative_share.numerator(), relative_share.denominator());
    let entitlement = total_share.checked_sub(prev_claim)?;

    to_json_binary(&NativeEntitlementResponse {
        amount: entitlement,
        denom,
    })
}

pub fn query_cw20_entitlement(deps: Deps, sender: Addr, token: String) -> StdResult<Binary> {
    let address = deps.api.addr_validate(sender.as_ref())?;
    let token = Addr::unchecked(token);

    let prev_claim = CW20_CLAIMS
        .may_load(deps.storage, (address, token.clone()))?
        .unwrap_or_default();
    let total_bal = CW20_BALANCES
        .may_load(deps.storage, token.clone())?
        .unwrap_or_default();
    let relative_share = get_relative_share(&deps, sender)?;

    let total_share =
        total_bal.multiply_ratio(relative_share.numerator(), relative_share.denominator());
    let entitlement = total_share.checked_sub(prev_claim)?;

    to_json_binary(&CW20EntitlementResponse {
        amount: entitlement,
        token_contract: token,
    })
}

pub fn query_native_entitlements(
    deps: Deps,
    sender: Addr,
    start_at: Option<String>,
    limit: Option<u32>,
) -> StdResult<Binary> {
    let address = deps.api.addr_validate(sender.as_ref())?;
    let relative_share = get_relative_share(&deps, sender)?;
    let natives = paginate_map(deps, &NATIVE_BALANCES, start_at, limit, Order::Descending)?;

    let mut entitlements: Vec<NativeEntitlementResponse> = vec![];
    for (denom, amount) in natives {
        let prev_claim = NATIVE_CLAIMS
            .may_load(deps.storage, (address.clone(), denom.clone()))?
            .unwrap_or_default();
        let total_share =
            amount.multiply_ratio(relative_share.numerator(), relative_share.denominator());
        let entitlement = total_share.checked_sub(prev_claim)?;

        entitlements.push(NativeEntitlementResponse {
            amount: entitlement,
            denom,
        });
    }

    to_json_binary(&entitlements)
}

pub fn query_cw20_entitlements(
    deps: Deps,
    sender: Addr,
    start_at: Option<String>,
    limit: Option<u32>,
) -> StdResult<Binary> {
    let address = deps.api.addr_validate(sender.as_ref())?;
    let relative_share = get_relative_share(&deps, sender)?;
    let start_at = start_at.map(|h| deps.api.addr_validate(&h)).transpose()?;
    let cw20s = paginate_map(deps, &CW20_BALANCES, start_at, limit, Order::Descending)?;

    let mut entitlements: Vec<CW20EntitlementResponse> = vec![];
    for (token, amount) in cw20s {
        let prev_claim = CW20_CLAIMS
            .may_load(deps.storage, (address.clone(), token.clone()))?
            .unwrap_or_default();

        let total_share =
            amount.multiply_ratio(relative_share.numerator(), relative_share.denominator());
        let entitlement = total_share.checked_sub(prev_claim)?;

        entitlements.push(CW20EntitlementResponse {
            amount: entitlement,
            token_contract: token,
        });
    }

    to_json_binary(&entitlements)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, msg: MigrateMsg) -> Result<Response, ContractError> {
    match msg {
        MigrateMsg::RedistributeUnclaimedFunds {
            distribution_height,
        } => execute_redistribute_unclaimed_funds(deps, distribution_height),
    }
}

// only cw_admin can call this
fn execute_redistribute_unclaimed_funds(
    deps: DepsMut,
    distribution_height: u64,
) -> Result<Response, ContractError> {
    // update the distribution height
    DISTRIBUTION_HEIGHT.save(deps.storage, &distribution_height)?;

    // get performed claims of cw20 and native tokens
    let performed_cw20_claims: Vec<Cw20ClaimEntry> = CW20_CLAIMS
        .range(deps.storage, None, None, Order::Descending)
        .collect();
    let performed_native_claims: Vec<NativeClaimEntry> = NATIVE_CLAIMS
        .range(deps.storage, None, None, Order::Descending)
        .collect();

    // subtract every performed claim from the available distributor balance
    for entry in performed_cw20_claims {
        let ((_, cw20_addr), amount) = entry?;
        CW20_BALANCES.update(deps.storage, cw20_addr.clone(), |bal| {
            // should never hit the None arm in theory
            match bal {
                Some(cw20_balance) => cw20_balance
                    .checked_sub(amount)
                    .map_err(ContractError::OverflowErr),
                None => Err(ContractError::Std(StdError::NotFound {
                    kind: cw20_addr.to_string(),
                })),
            }
        })?;
    }

    // subtract every performed claim from the available distributor balance
    for entry in performed_native_claims {
        let ((_, denom), amount) = entry?;
        NATIVE_BALANCES.update(deps.storage, denom.clone(), |bal| {
            // should never hit the None arm in theory
            match bal {
                Some(native_balance) => native_balance
                    .checked_sub(amount)
                    .map_err(ContractError::OverflowErr),
                None => Err(ContractError::Std(StdError::NotFound {
                    kind: denom.to_string(),
                })),
            }
        })?;
    }

    // nullify previous claims
    CW20_CLAIMS.clear(deps.storage);
    NATIVE_CLAIMS.clear(deps.storage);

    Ok(Response::default().add_attribute("method", "redistribute_unclaimed_funds"))
}
