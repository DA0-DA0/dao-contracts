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
    to_binary, Addr, BankMsg, Binary, Coin, Decimal, Deps, DepsMut, Env, Fraction, MessageInfo,
    Order, Response, StdError, StdResult, Uint128, WasmMsg,
};
use cw2::set_contract_version;
use cw_paginate::paginate_map;
use std::borrow::Borrow;
use std::collections::HashMap;

use dao_interface::voting;

const CONTRACT_NAME: &str = "crates.io:cw-fund-distributor";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

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

    let balance = CW20_BALANCES.may_load(deps.storage, token.clone())?;

    match balance {
        Some(old_amount) => CW20_BALANCES.save(
            deps.storage,
            token.clone(),
            &old_amount.checked_add(amount).unwrap(),
        )?,
        None => CW20_BALANCES.save(deps.storage, token.clone(), &amount)?,
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
            let balance = NATIVE_BALANCES
                .may_load(deps.storage, coin.denom.clone())
                .unwrap_or_default();

            let new_amount = coin.amount;
            // add any previous balances
            if let Some(previous_amount) = balance {
                new_amount
                    .checked_add(previous_amount)
                    .map_err(|e| ContractError::Std(StdError::from(e)))?;
            };
            NATIVE_BALANCES.save(deps.storage, coin.denom.clone(), &new_amount)?;
            attributes.push((coin.denom, new_amount.to_string()));
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
) -> Uint128 {
    distributor_funds
        .multiply_ratio(relative_share.numerator(), relative_share.denominator())
        .checked_sub(previous_claim)
        .unwrap()
}

fn get_relative_share(deps: &Deps, sender: Addr) -> Decimal {
    let voting_contract = VOTING_CONTRACT.load(deps.storage).unwrap();
    let dist_height = DISTRIBUTION_HEIGHT.load(deps.storage).unwrap();
    let total_power = TOTAL_POWER.load(deps.storage).unwrap();

    // find the voting power of sender at distributor instantiation
    let voting_power: voting::VotingPowerAtHeightResponse = deps
        .querier
        .query_wasm_smart(
            voting_contract,
            &voting::Query::VotingPowerAtHeight {
                address: sender.to_string(),
                height: Some(dist_height),
            },
        )
        .unwrap();
    // return senders share
    Decimal::from_ratio(voting_power.power, total_power)
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

    let relative_share = get_relative_share(deps.as_ref().borrow(), sender.clone());

    let messages: Vec<WasmMsg> = tokens
        .into_iter()
        .map(|addr| {
            // get the balance of distributor at instantiation
            let bal = CW20_BALANCES
                .load(deps.storage, Addr::unchecked(addr.clone()))
                .unwrap();
            // check for any previous claims
            let previous_claim = CW20_CLAIMS
                .load(
                    deps.storage,
                    (sender.clone(), Addr::unchecked(addr.clone())),
                )
                .unwrap_or_default();

            // get % share of sender and subtract any previous claims
            let entitlement = get_entitlement(bal, relative_share, previous_claim);

            // reflect the new total claim amount
            CW20_CLAIMS
                .update(
                    deps.storage,
                    (sender.clone(), Addr::unchecked(addr.clone())),
                    |claim| {
                        claim
                            .unwrap_or_default()
                            .checked_add(entitlement)
                            .map_err(StdError::overflow)
                    },
                )
                .unwrap();

            // add the transfer message
            (
                WasmMsg::Execute {
                    contract_addr: addr,
                    msg: to_binary(&cw20::Cw20ExecuteMsg::Transfer {
                        recipient: sender.to_string(),
                        amount: entitlement,
                    })
                    .unwrap(),
                    funds: vec![],
                },
                entitlement,
            )
        })
        // filter out zero entitlement messages
        .filter(|(_, entitlement)| !entitlement.is_zero())
        .map(|(msg, _)| msg)
        .collect();

    Ok(Response::default()
        .add_attribute("method", "claim_cw20s")
        .add_attribute("sender", sender)
        .add_messages(messages))
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

    let relative_share = get_relative_share(deps.as_ref().borrow(), sender.clone());

    let messages: Vec<_> = denoms
        .into_iter()
        .map(|addr| {
            // get the balance of distributor at instantiation
            let bal = NATIVE_BALANCES.load(deps.storage, addr.clone()).unwrap();

            // check for any previous claims
            let previous_claim = NATIVE_CLAIMS
                .load(deps.storage, (sender.clone(), addr.clone()))
                .unwrap_or_default();

            // get % share of sender and subtract any previous claims
            let entitlement = get_entitlement(bal, relative_share, previous_claim);

            // reflect the new total claim amount
            NATIVE_CLAIMS
                .update(deps.storage, (sender.clone(), addr.clone()), |claim| {
                    claim
                        .unwrap_or_default()
                        .checked_add(entitlement)
                        .map_err(StdError::overflow)
                })
                .unwrap();

            // collect the transfer messages
            (
                BankMsg::Send {
                    to_address: sender.to_string(),
                    amount: vec![Coin {
                        denom: addr,
                        amount: entitlement,
                    }],
                },
                entitlement,
            )
        })
        // filter out zero entitlement messages
        .filter(|(_, entitlement)| !entitlement.is_zero())
        .map(|(msg, _)| msg)
        .collect();

    Ok(Response::default()
        .add_attribute("method", "claim_natives")
        .add_attribute("sender", sender)
        .add_messages(messages))
}

pub fn execute_claim_all(deps: DepsMut, env: Env, sender: Addr) -> Result<Response, ContractError> {
    let funding_deadline = FUNDING_PERIOD_EXPIRATION.load(deps.storage)?;
    // if current block indicates funding period, throw
    if !funding_deadline.is_expired(&env.block) {
        return Err(ContractError::ClaimDuringFundingPeriod {});
    }
    let relative_share = get_relative_share(deps.as_ref().borrow(), sender.clone());

    let cw20s: Vec<(Addr, Uint128)> = CW20_BALANCES
        .range(deps.storage, None, None, Order::Descending)
        .map(|cw20| cw20.unwrap())
        .collect();

    let natives: Vec<(String, Uint128)> = NATIVE_BALANCES
        .range(deps.storage, None, None, Order::Descending)
        .map(|native| native.unwrap())
        .collect();

    // collect transfer messages and update store
    let cw20_transfer_msgs: Vec<WasmMsg> = cw20s
        .into_iter()
        .map(|(addr, amount)| {
            let previous_claim = CW20_CLAIMS
                .load(deps.storage, (sender.clone(), addr.clone()))
                .unwrap_or_default();

            // get % share of sender and subtract any previous claims
            let entitlement = get_entitlement(amount, relative_share, previous_claim);

            // reflect the new total claim amount
            CW20_CLAIMS
                .update(deps.storage, (sender.clone(), addr.clone()), |claim| {
                    claim
                        .unwrap_or_default()
                        .checked_add(entitlement)
                        .map_err(StdError::overflow)
                })
                .unwrap();

            (
                WasmMsg::Execute {
                    contract_addr: addr.to_string(),
                    msg: to_binary(&cw20::Cw20ExecuteMsg::Transfer {
                        recipient: sender.to_string(),
                        amount: entitlement,
                    })
                    .unwrap(),
                    funds: vec![],
                },
                entitlement,
            )
        })
        // filter out zero entitlement messages
        .filter(|(_, entitlement)| !entitlement.is_zero())
        .map(|(msg, _)| msg)
        .collect();

    let native_transfer_msgs: Vec<BankMsg> = natives
        .into_iter()
        .map(|(denom, amount)| {
            let previous_claim = NATIVE_CLAIMS
                .load(deps.storage, (sender.clone(), denom.clone()))
                .unwrap_or_default();

            // get % share of sender and subtract any previous claims
            let entitlement = get_entitlement(amount, relative_share, previous_claim);

            // reflect the new total claim amount
            NATIVE_CLAIMS
                .update(deps.storage, (sender.clone(), denom.clone()), |claim| {
                    claim
                        .unwrap_or_default()
                        .checked_add(entitlement)
                        .map_err(StdError::overflow)
                })
                .unwrap();

            // add the transfer message
            (
                BankMsg::Send {
                    to_address: sender.to_string(),
                    amount: vec![Coin {
                        denom,
                        amount: entitlement,
                    }],
                },
                entitlement,
            )
        })
        // filter out zero entitlement messages
        .filter(|(_, entitlement)| !entitlement.is_zero())
        .map(|(msg, _)| msg)
        .collect();

    Ok(Response::default()
        .add_messages(cw20_transfer_msgs)
        .add_messages(native_transfer_msgs)
        .add_attribute("method", "claim_all"))
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
    to_binary(&VotingContractResponse {
        contract,
        distribution_height,
    })
}

pub fn query_total_power(deps: Deps) -> StdResult<Binary> {
    let total_power = TOTAL_POWER.load(deps.storage)?;
    to_binary(&TotalPowerResponse { total_power })
}

pub fn query_native_denoms(deps: Deps) -> StdResult<Binary> {
    let native_balances = NATIVE_BALANCES.range(deps.storage, None, None, Order::Ascending);

    let denom_responses: Vec<DenomResponse> = native_balances
        .into_iter()
        .map(|denom| denom.unwrap())
        .map(|(denom, amount)| DenomResponse {
            contract_balance: amount,
            denom,
        })
        .collect();
    to_binary(&denom_responses)
}

pub fn query_cw20_tokens(deps: Deps) -> StdResult<Binary> {
    let cw20_balances = CW20_BALANCES.range(deps.storage, None, None, Order::Ascending);

    let cw20_responses: Vec<CW20Response> = cw20_balances
        .into_iter()
        .map(|cw20| cw20.unwrap())
        .map(|(token, amount)| CW20Response {
            contract_balance: amount,
            token: token.to_string(),
        })
        .collect();
    to_binary(&cw20_responses)
}

pub fn query_native_entitlement(deps: Deps, sender: Addr, denom: String) -> StdResult<Binary> {
    let address = deps.api.addr_validate(sender.as_ref())?;
    let prev_claim = NATIVE_CLAIMS.load(deps.storage, (address, denom.clone()))?;
    let total_bal = NATIVE_BALANCES.load(deps.storage, denom.clone())?;
    let relative_share = get_relative_share(&deps, sender);

    let entitlement = get_entitlement(total_bal, relative_share, prev_claim);

    to_binary(&NativeEntitlementResponse {
        amount: entitlement,
        denom,
    })
}

pub fn query_cw20_entitlement(deps: Deps, sender: Addr, token: String) -> StdResult<Binary> {
    let address = deps.api.addr_validate(sender.as_ref())?;
    let token = Addr::unchecked(token);
    let prev_claim = CW20_CLAIMS.load(deps.storage, (address, token.clone()))?;
    let total_bal = CW20_BALANCES.load(deps.storage, token.clone())?;
    let relative_share = get_relative_share(&deps, sender);
    let entitlement = get_entitlement(total_bal, relative_share, prev_claim);

    to_binary(&CW20EntitlementResponse {
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
    let relative_share = get_relative_share(&deps, sender);
    let natives = paginate_map(deps, &NATIVE_BALANCES, start_at, limit, Order::Descending)?;

    let entitlements: Vec<NativeEntitlementResponse> = natives
        .into_iter()
        .map(|(denom, amount)| {
            let prev_claim = NATIVE_CLAIMS
                .load(deps.storage, (address.clone(), denom.clone()))
                .unwrap();
            NativeEntitlementResponse {
                amount: get_entitlement(amount, relative_share, prev_claim),
                denom,
            }
        })
        .collect();

    to_binary(&entitlements)
}

pub fn query_cw20_entitlements(
    deps: Deps,
    sender: Addr,
    start_at: Option<String>,
    limit: Option<u32>,
) -> StdResult<Binary> {
    let address = deps.api.addr_validate(sender.as_ref())?;
    let relative_share = get_relative_share(&deps, sender);
    let start_at = start_at.map(|h| deps.api.addr_validate(&h)).transpose()?;
    let cw20s = paginate_map(deps, &CW20_BALANCES, start_at, limit, Order::Descending)?;

    let entitlements: Vec<CW20EntitlementResponse> = cw20s
        .into_iter()
        .map(|(token, amount)| {
            let prev_claim = CW20_CLAIMS
                .load(deps.storage, (address.clone(), token.clone()))
                .unwrap();
            CW20EntitlementResponse {
                amount: get_entitlement(amount, relative_share, prev_claim),
                token_contract: token,
            }
        })
        .collect();

    to_binary(&entitlements)
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
    let performed_cw20_claims: HashMap<(Addr, Addr), Uint128> = CW20_CLAIMS
        .range(deps.storage, None, None, Order::Descending)
        .map(|native| native.unwrap())
        .collect();

    let performed_native_claims: HashMap<(Addr, String), Uint128> = NATIVE_CLAIMS
        .range(deps.storage, None, None, Order::Descending)
        .map(|native| native.unwrap())
        .collect();

    // subtract the performed claim amounts from
    // balances available for claiming
    performed_native_claims
        .into_iter()
        .for_each(|((_, denom), amount)| {
            NATIVE_BALANCES
                .update(deps.storage, denom, |bal| {
                    bal.unwrap_or_default()
                        .checked_sub(amount)
                        .map_err(StdError::overflow)
                })
                .unwrap();
        });

    performed_cw20_claims
        .into_iter()
        .for_each(|((_, cw20_addr), amount)| {
            CW20_BALANCES
                .update(deps.storage, cw20_addr, |bal| {
                    bal.unwrap_or_default()
                        .checked_sub(amount)
                        .map_err(StdError::overflow)
                })
                .unwrap();
        });

    // nullify previous claims
    CW20_CLAIMS.clear(deps.storage);
    NATIVE_CLAIMS.clear(deps.storage);

    Ok(Response::default().add_attribute("method", "redistribute_unclaimed_funds"))
}
