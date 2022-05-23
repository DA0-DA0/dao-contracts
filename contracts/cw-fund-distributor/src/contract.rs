use std::convert::TryInto;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, BankMsg, Binary, Coin, Deps, DepsMut, Env, MessageInfo, Response, StdError,
    StdResult, Uint128, Uint256, WasmMsg,
};
use cw2::set_contract_version;

use cw_core_interface::voting;
use cw_storage_plus::{Bound, Map, PrimaryKey};

use crate::error::ContractError;
use crate::msg::{AdminResponse, DenomResponse, ExecuteMsg, InstantiateMsg, QueryMsg};
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
    VOTING_CONTRACT.save(deps.storage, &&voting_contract)?;

    Ok(Response::default()
        .add_attribute(
            "admin",
            admin.map(|a| a.into_string()).unwrap_or("None".to_string()),
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
    _env: Env,
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
        ExecuteMsg::WithdrawCw20s { tokens: _ } => todo!(),
        ExecuteMsg::WithdrawNatives { denoms: _ } => todo!(),
        ExecuteMsg::UpdateAdmin { admin: _ } => todo!(),
    }
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

pub fn compute_entitlements<'a, K>(
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
            let entitled = total_entitlement - claimed;
            Ok((key, entitled))
        })
        .collect::<StdResult<Vec<_>>>()
}

pub fn execute_claim_cw20s(
    deps: DepsMut,
    sender: Addr,
    tokens: Option<Vec<String>>,
) -> Result<Response, ContractError> {
    let tokens: Vec<(Addr, Uint128)> = match tokens {
        Some(tokens) => tokens
            .into_iter()
            .map(|h| deps.api.addr_validate(&h))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .map(|a| {
                let amount = CW20S.load(deps.storage, a.clone())?;
                Ok((a, amount))
            })
            .collect::<Result<_, StdError>>()?,
        None => CW20S
            .range(deps.storage, None, None, cosmwasm_std::Order::Descending)
            .collect::<Result<_, _>>()?,
    };

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
    let denoms: Vec<(String, Uint128)> = match denoms {
        Some(denoms) => denoms
            .into_iter()
            .map(|a| {
                let amount = NATIVES.load(deps.storage, a.clone())?;
                Ok((a, amount))
            })
            .collect::<Result<_, StdError>>()?,
        None => NATIVES
            .range(deps.storage, None, None, cosmwasm_std::Order::Descending)
            .collect::<Result<_, _>>()?,
    };

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

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::NativeDenoms { start_at, limit } => query_native_denoms(deps, start_at, limit),
        QueryMsg::Cw20Denoms { start_at, limit } => query_cw20_denoms(deps, start_at, limit),
        QueryMsg::NativeEntitlement { address, denom } => todo!(),
        QueryMsg::Cw20Entitlement { address, token } => todo!(),
        QueryMsg::NativeEntitlements {
            address,
            start_at,
            limit,
        } => todo!(),
        QueryMsg::Cw20Entitlements {
            address,
            start_at,
            limit,
        } => todo!(),
        QueryMsg::Admin {} => query_admin(deps),
    }
}

pub fn query_admin(deps: Deps) -> StdResult<Binary> {
    to_binary(&AdminResponse {
        admin: ADMIN.load(deps.storage)?,
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
