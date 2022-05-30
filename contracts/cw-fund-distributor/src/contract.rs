use std::convert::TryInto;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, BankMsg, Binary, Coin, Deps, DepsMut, Env, MessageInfo, Response, StdError,
    StdResult, Uint128, Uint256, WasmMsg,
};
use cw2::set_contract_version;

use cw_core_interface::voting;
use cw_storage_plus::{Bound, Bounder, KeyDeserialize, Map, PrimaryKey};

use crate::error::ContractError;
use crate::msg::{
    AdminResponse, Cw20EntitlementResponse, DenomResponse, ExecuteMsg, InstantiateMsg,
    NativeEntitlementResponse, QueryMsg, VotingContractResponse,
};
use crate::state::{
    ADMIN, CW20S, CW20_CLAIMS, DISTRIBUTION_HEIGHT, NATIVES, NATIVE_CLAIMS, TOTAL_POWER,
    VOTING_CONTRACT,
};

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

    if msg.distribution_height > env.block.height {
        return Err(ContractError::DistributionHeight {});
    }
    DISTRIBUTION_HEIGHT.save(deps.storage, &msg.distribution_height)?;

    let admin = msg.admin.map(|a| deps.api.addr_validate(&a)).transpose()?;
    ADMIN.save(deps.storage, &admin)?;

    let voting_contract = deps.api.addr_validate(&msg.voting_contract)?;

    let total_power: voting::TotalPowerAtHeightResponse = deps.querier.query_wasm_smart(
        voting_contract.clone(),
        &voting::Query::TotalPowerAtHeight {
            height: Some(msg.distribution_height),
        },
    )?;

    if total_power.power.is_zero() {
        return Err(ContractError::ZeroTotalPower {});
    }

    TOTAL_POWER.save(deps.storage, &total_power.power)?;
    VOTING_CONTRACT.save(deps.storage, &voting_contract)?;

    Ok(Response::default()
        .add_attribute(
            "admin",
            admin
                .map(|a| a.into_string())
                .unwrap_or_else(|| "None".to_string()),
        )
        .add_attribute(
            "distribution_height",
            format!("{}", msg.distribution_height),
        )
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
        }) => execute_receive_cw20(deps, info.sender, amount),
        ExecuteMsg::Fund {} => execute_fund_natives(deps, info),
        ExecuteMsg::ClaimCw20s { tokens } => execute_claim_cw20s(deps, info.sender, tokens),
        ExecuteMsg::ClaimNatives { denoms } => execute_claim_natives(deps, info.sender, denoms),
        ExecuteMsg::WithdrawCw20s { tokens } => {
            execute_withdraw_cw20s(deps, env, info.sender, tokens)
        }
        ExecuteMsg::WithdrawNatives { denoms } => {
            execute_withdraw_natives(deps, env, info.sender, denoms)
        }
        ExecuteMsg::UpdateAdmin { admin } => execute_update_admin(deps, info.sender, admin),
    }
}

/// Computes the total entitlement for the provided set of
/// paramater. NOTE: this may differ than the actual amount that an
/// address is entitled to if the address has already claimed some of
/// their entitlement.
fn compute_entitled(
    provided: Uint128,
    voting_power: Uint128,
    total_power: Uint128,
) -> StdResult<Uint128> {
    Ok(provided
        .full_mul(voting_power)
        .checked_div(Uint256::from(total_power))?
        .try_into()
        .unwrap())
}

/// Generic function for computing the amount of tokens an address is
/// entitled to for each denom being distributed.
fn compute_entitlements<'a, K>(
    deps: Deps,
    contract_balances: Vec<(K, Uint128)>,
    sender: Addr,
    claims: Map<'a, (Addr, K), Uint128>,
) -> StdResult<Vec<(K, Uint128)>>
where
    (Addr, K): PrimaryKey<'a>,
    K: Clone,
{
    let total_power = TOTAL_POWER.load(deps.storage)?;
    let dist_height = DISTRIBUTION_HEIGHT.load(deps.storage)?;
    let voting_contract = VOTING_CONTRACT.load(deps.storage)?;
    let voting_power: voting::VotingPowerAtHeightResponse = deps.querier.query_wasm_smart(
        voting_contract,
        &voting::Query::VotingPowerAtHeight {
            address: sender.to_string(),
            height: Some(dist_height),
        },
    )?;

    contract_balances
        .into_iter()
        .map(|(key, provided)| {
            let total_entitlement = compute_entitled(provided, voting_power.power, total_power)?;
            let claimed = claims
                .may_load(deps.storage, (sender.clone(), key.clone()))?
                .unwrap_or_default();
            // If funds have been provided, withdrawn, and
            // subsequentally provided again it is possible that there
            // are accounts that have claimied more than they are
            // currently entitled to. This happens if the initial
            // amount provided was greater than the amount provided
            // after withdrawal and the account claimed while the
            // initial amount was up.
            let entitled = if claimed > total_entitlement {
                Uint128::zero()
            } else {
                total_entitlement - claimed
            };
            Ok((key, entitled))
        })
        .collect::<StdResult<Vec<_>>>()
}

/// Generic function for paginating a list of (K, V) pairs in a
/// CosmWasm Map.
fn paginate_items<'a, K, V>(
    deps: Deps,
    map: Map<'a, K, V>,
    start_at: Option<K>,
    limit: Option<u32>,
) -> StdResult<Vec<(K, V)>>
where
    K: Bounder<'a> + KeyDeserialize<Output = K> + 'static,
    V: serde::de::DeserializeOwned + serde::Serialize,
{
    let items = map.range(
        deps.storage,
        start_at.map(Bound::inclusive),
        None,
        cosmwasm_std::Order::Descending,
    );
    match limit {
        Some(limit) => Ok(items.take(limit as usize).collect::<StdResult<_>>()?),
        None => Ok(items.collect::<StdResult<_>>()?),
    }
}

/// Generic function for listing token balance pairs optionally given
/// some subset of pairs that should be looked up.
fn list_token_balance_pairs<'a, K>(
    deps: Deps,
    items: Map<'a, K, Uint128>,
    lookup: Option<Vec<K>>,
) -> StdResult<Vec<(K, Uint128)>>
where
    K: KeyDeserialize<Output = K> + Bounder<'a> + 'static + Clone,
{
    match lookup {
        Some(lookup) => lookup
            .into_iter()
            .map(|k| Ok((k.clone(), items.load(deps.storage, k)?)))
            .collect::<StdResult<_>>(),
        None => paginate_items(deps, items, None, None),
    }
}

pub fn execute_update_admin(
    deps: DepsMut,
    sender: Addr,
    new_admin: Option<String>,
) -> Result<Response, ContractError> {
    let new_admin = new_admin.map(|h| deps.api.addr_validate(&h)).transpose()?;
    let admin = ADMIN.load(deps.storage)?;
    if Some(sender) != admin {
        return Err(ContractError::Unauthorized {});
    }
    ADMIN.save(deps.storage, &new_admin)?;

    Ok(Response::default()
        .add_attribute("method", "update_admin")
        .add_attribute(
            "new_admin",
            new_admin
                .map(|a| a.into_string())
                .unwrap_or_else(|| "None".to_string()),
        ))
}

pub fn execute_receive_cw20(
    deps: DepsMut,
    token_contract: Addr,
    amount: Uint128,
) -> Result<Response, ContractError> {
    // Should never hit this, but you really never know what sort of
    // cw20 contract you're dealing with.
    if amount.is_zero() {
        return Err(ContractError::ZeroFunds {});
    }

    CW20S.update(
        deps.storage,
        token_contract.clone(),
        |v| -> StdResult<Uint128> {
            match v {
                // It is possible this will overflow in the event that
                // a cw20 token has been provided, distributed, and
                // then provied again. In that case this will error
                // and a new rewards contract will need to be created
                // for distributing those tokens.
                Some(old_amount) => old_amount.checked_add(amount).map_err(StdError::overflow),
                None => Ok(amount),
            }
        },
    )?;

    Ok(Response::default()
        .add_attribute("method", "receive_cw20")
        .add_attribute("token", token_contract)
        .add_attribute("amount", amount))
}

pub fn execute_fund_natives(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    let mut response = Response::default().add_attribute("method", "fund_natives");

    for Coin { amount, denom } in info.funds.into_iter() {
        response = response.add_attribute(denom.clone(), amount);
        NATIVES.update(deps.storage, denom, |v| -> StdResult<Uint128> {
            match v {
                Some(old_amount) => old_amount.checked_add(amount).map_err(StdError::overflow),
                None => Ok(amount),
            }
        })?;
    }

    Ok(response)
}

pub fn execute_claim_cw20s(
    deps: DepsMut,
    sender: Addr,
    tokens: Option<Vec<String>>,
) -> Result<Response, ContractError> {
    let tokens = list_token_balance_pairs(
        deps.as_ref(),
        CW20S,
        tokens
            .map(|tokens| {
                tokens
                    .into_iter()
                    .map(|h| deps.api.addr_validate(&h))
                    .collect::<StdResult<Vec<_>>>()
            })
            .transpose()?,
    )?;

    let entitlements = compute_entitlements(deps.as_ref(), tokens, sender.clone(), CW20_CLAIMS)?;
    let messages = entitlements
        .into_iter()
        .filter(|(_, entitled)| !entitled.is_zero())
        .map(|(addr, entitled)| {
            CW20_CLAIMS.update(deps.storage, (sender.clone(), addr.clone()), |claimed| {
                claimed
                    .unwrap_or_default()
                    .checked_add(entitled)
                    .map_err(StdError::overflow)
            })?;
            Ok(WasmMsg::Execute {
                contract_addr: addr.into_string(),
                msg: to_binary(&cw20::Cw20ExecuteMsg::Transfer {
                    recipient: sender.to_string(),
                    amount: entitled,
                })?,
                funds: vec![],
            })
        })
        .collect::<StdResult<Vec<_>>>()?;

    Ok(Response::default()
        .add_messages(messages)
        .add_attribute("method", "claim_cw20s")
        .add_attribute("sender", sender))
}

pub fn execute_claim_natives(
    deps: DepsMut,
    sender: Addr,
    denoms: Option<Vec<String>>,
) -> Result<Response, ContractError> {
    let denoms = list_token_balance_pairs(deps.as_ref(), NATIVES, denoms)?;

    let entitlements = compute_entitlements(deps.as_ref(), denoms, sender.clone(), NATIVE_CLAIMS)?;
    let coins = entitlements
        .into_iter()
        .filter(|(_, entitled)| !entitled.is_zero())
        .map(|(denom, entitled)| {
            NATIVE_CLAIMS.update(deps.storage, (sender.clone(), denom.clone()), |claimed| {
                claimed
                    .unwrap_or_default()
                    .checked_add(entitled)
                    .map_err(StdError::overflow)
            })?;
            Ok(Coin {
                denom,
                amount: entitled,
            })
        })
        .collect::<StdResult<Vec<_>>>()?;

    let message = BankMsg::Send {
        to_address: sender.to_string(),
        amount: coins,
    };

    Ok(Response::default()
        .add_message(message)
        .add_attribute("method", "claim_natives")
        .add_attribute("sender", sender))
}

pub fn execute_withdraw_cw20s(
    deps: DepsMut,
    env: Env,
    sender: Addr,
    tokens: Option<Vec<String>>,
) -> Result<Response, ContractError> {
    let admin = ADMIN.load(deps.storage)?;
    let admin = if admin != Some(sender) {
        return Err(ContractError::Unauthorized {});
    } else {
        // Safe to unwrap here as we checked `Some(sender)` above.
        admin.unwrap()
    };

    let tokens = list_token_balance_pairs(
        deps.as_ref(),
        CW20S,
        tokens
            .map(|tokens| {
                tokens
                    .into_iter()
                    .map(|h| deps.api.addr_validate(&h))
                    .collect::<StdResult<Vec<_>>>()
            })
            .transpose()?,
    )?;
    let messages = tokens
        .into_iter()
        .map(|(token_contract, _)| {
            CW20S.remove(deps.storage, token_contract.clone());

            let remaining: cw20::BalanceResponse = deps.querier.query_wasm_smart(
                token_contract.clone(),
                &cw20::Cw20QueryMsg::Balance {
                    address: env.contract.address.to_string(),
                },
            )?;

            Ok(WasmMsg::Execute {
                contract_addr: token_contract.into_string(),
                msg: to_binary(&cw20::Cw20ExecuteMsg::Transfer {
                    recipient: admin.to_string(),
                    amount: remaining.balance,
                })?,
                funds: vec![],
            })
        })
        .collect::<StdResult<Vec<_>>>()?;

    Ok(Response::default()
        .add_attribute("method", "withdraw_natives")
        .add_messages(messages))
}

pub fn execute_withdraw_natives(
    deps: DepsMut,
    env: Env,
    sender: Addr,
    denoms: Option<Vec<String>>,
) -> Result<Response, ContractError> {
    let admin = ADMIN.load(deps.storage)?;
    let admin = if admin != Some(sender) {
        return Err(ContractError::Unauthorized {});
    } else {
        // Safe to unwrap here as we checked `Some(sender)` above.
        admin.unwrap()
    };

    let denoms = list_token_balance_pairs(deps.as_ref(), NATIVES, denoms)?;
    let coins = denoms
        .into_iter()
        .map(|(denom, _)| {
            NATIVES.remove(deps.storage, denom.clone());

            let remaining = deps
                .querier
                .query_balance(env.contract.address.to_string(), denom.clone())?;

            Ok(Coin {
                amount: remaining.amount,
                denom,
            })
        })
        .collect::<StdResult<Vec<_>>>()?;

    let msg = BankMsg::Send {
        to_address: admin.into_string(),
        amount: coins,
    };

    Ok(Response::default()
        .add_attribute("method", "withdraw_natives")
        .add_message(msg))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::NativeDenoms { start_at, limit } => query_native_denoms(deps, start_at, limit),
        QueryMsg::Cw20Denoms { start_at, limit } => query_cw20_denoms(deps, start_at, limit),
        QueryMsg::NativeEntitlement { address, denom } => {
            query_native_entitlement(deps, address, denom)
        }
        QueryMsg::Cw20Entitlement { address, token } => {
            query_cw20_entitlement(deps, address, token)
        }
        QueryMsg::NativeEntitlements {
            address,
            start_at,
            limit,
        } => query_native_entitlements(deps, address, start_at, limit),
        QueryMsg::Cw20Entitlements {
            address,
            start_at,
            limit,
        } => query_cw20_entitlements(deps, address, start_at, limit),
        QueryMsg::Admin {} => query_admin(deps),
        QueryMsg::VotingContract {} => query_voting_contract(deps),
    }
}

pub fn query_native_entitlements(
    deps: Deps,
    address: String,
    start_at: Option<String>,
    limit: Option<u32>,
) -> StdResult<Binary> {
    let address = deps.api.addr_validate(&address)?;
    let natives = paginate_items(deps, NATIVES, start_at, limit)?;

    let entitlements = compute_entitlements(deps, natives, address, NATIVE_CLAIMS)?;

    to_binary(
        &entitlements
            .into_iter()
            .map(|(denom, amount)| NativeEntitlementResponse { amount, denom })
            .collect::<Vec<_>>(),
    )
}

pub fn query_cw20_entitlements(
    deps: Deps,
    address: String,
    start_at: Option<String>,
    limit: Option<u32>,
) -> StdResult<Binary> {
    let start_at = start_at.map(|h| deps.api.addr_validate(&h)).transpose()?;
    let address = deps.api.addr_validate(&address)?;

    let cw20s = paginate_items(deps, CW20S, start_at, limit)?;
    let entitlements = compute_entitlements(deps, cw20s, address, CW20_CLAIMS)?;

    to_binary(
        &entitlements
            .into_iter()
            .map(|(token_contract, amount)| Cw20EntitlementResponse {
                amount,
                token_contract,
            })
            .collect::<Vec<_>>(),
    )
}

pub fn query_native_entitlement(deps: Deps, address: String, denom: String) -> StdResult<Binary> {
    let address = deps.api.addr_validate(&address)?;

    let total_power = TOTAL_POWER.load(deps.storage)?;
    let dist_height = DISTRIBUTION_HEIGHT.load(deps.storage)?;
    let voting_contract = VOTING_CONTRACT.load(deps.storage)?;
    let voting_power: voting::VotingPowerAtHeightResponse = deps.querier.query_wasm_smart(
        voting_contract,
        &voting::Query::VotingPowerAtHeight {
            address: address.to_string(),
            height: Some(dist_height),
        },
    )?;

    let provided = NATIVES.load(deps.storage, denom.clone())?;
    let total_entitlement = compute_entitled(provided, voting_power.power, total_power)?;
    let claimed = NATIVE_CLAIMS.load(deps.storage, (address, denom.clone()))?;

    to_binary(&NativeEntitlementResponse {
        amount: total_entitlement - claimed,
        denom,
    })
}

pub fn query_cw20_entitlement(
    deps: Deps,
    address: String,
    token_contract: String,
) -> StdResult<Binary> {
    let address = deps.api.addr_validate(&address)?;
    let token_contract = deps.api.addr_validate(&token_contract)?;

    let total_power = TOTAL_POWER.load(deps.storage)?;
    let dist_height = DISTRIBUTION_HEIGHT.load(deps.storage)?;
    let voting_contract = VOTING_CONTRACT.load(deps.storage)?;
    let voting_power: voting::VotingPowerAtHeightResponse = deps.querier.query_wasm_smart(
        voting_contract,
        &voting::Query::VotingPowerAtHeight {
            address: address.to_string(),
            height: Some(dist_height),
        },
    )?;

    let provided = CW20S.load(deps.storage, token_contract.clone())?;
    let total_entitlement = compute_entitled(provided, voting_power.power, total_power)?;
    let claimed = CW20_CLAIMS.load(deps.storage, (address, token_contract.clone()))?;

    let amount = if claimed > total_entitlement {
        Uint128::zero()
    } else {
        total_entitlement - claimed
    };

    to_binary(&Cw20EntitlementResponse {
        amount,
        token_contract,
    })
}

pub fn query_admin(deps: Deps) -> StdResult<Binary> {
    to_binary(&AdminResponse {
        admin: ADMIN.load(deps.storage)?,
    })
}

pub fn query_voting_contract(deps: Deps) -> StdResult<Binary> {
    let contract = VOTING_CONTRACT.load(deps.storage)?;
    let distribution_height = DISTRIBUTION_HEIGHT.load(deps.storage)?;
    to_binary(&VotingContractResponse {
        contract,
        distribution_height,
    })
}

pub fn query_native_denoms(
    deps: Deps,
    start_at: Option<String>,
    limit: Option<u32>,
) -> StdResult<Binary> {
    let natives = NATIVES.range(
        deps.storage,
        start_at.map(Bound::inclusive),
        None,
        cosmwasm_std::Order::Descending,
    );
    let natives: Vec<(String, Uint128)> = match limit {
        Some(limit) => natives.take(limit as usize).collect(),
        None => natives.collect::<Result<_, StdError>>(),
    }?;
    let response: Vec<_> = natives
        .into_iter()
        .map(|(denom, contract_balance)| DenomResponse {
            contract_balance,
            denom,
        })
        .collect();

    to_binary(&response)
}

pub fn query_cw20_denoms(
    deps: Deps,
    start_at: Option<String>,
    limit: Option<u32>,
) -> StdResult<Binary> {
    let start_at = start_at.map(|h| deps.api.addr_validate(&h)).transpose()?;
    let cw20s = CW20S.range(
        deps.storage,
        start_at.map(Bound::inclusive),
        None,
        cosmwasm_std::Order::Descending,
    );
    let cw20s: Vec<(Addr, Uint128)> = match limit {
        Some(limit) => cw20s.take(limit as usize).collect(),
        None => cw20s.collect::<Result<_, StdError>>(),
    }?;
    let response: Vec<_> = cw20s
        .into_iter()
        .map(|(denom, contract_balance)| DenomResponse {
            contract_balance,
            denom: denom.into_string(),
        })
        .collect();

    to_binary(&response)
}
