use std::cmp::min;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Addr, CosmosMsg, StdError, Uint128, WasmMsg};

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InfoResponse, InstantiateMsg, QueryMsg};
use crate::state::{Config, CONFIG, LAST_PAYMENT_BLOCK};
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw2::set_contract_version;

const CONTRACT_NAME: &str = "crates.io:stake-cw20-reward-distributor";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let owner = deps.api.addr_validate(&msg.owner)?;
    let staking_addr = deps.api.addr_validate(&msg.staking_addr)?;
    if !validate_staking(deps.as_ref(), staking_addr.clone()) {
        return Err(ContractError::InvalidStakingContract {});
    }

    let reward_token = deps.api.addr_validate(&msg.reward_token)?;
    if !validate_cw20(deps.as_ref(), reward_token.clone()) {
        return Err(ContractError::InvalidCw20 {});
    }

    let config = Config {
        owner,
        staking_addr,
        reward_token,
        reward_rate: msg.reward_rate,
    };
    CONFIG.save(deps.storage, &config)?;

    // Initialize last payment block
    LAST_PAYMENT_BLOCK.save(deps.storage, &env.block.height)?;

    Ok(Response::new().add_attribute("action", "instantiate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateConfig {
            owner,
            staking_addr,
            reward_rate,
            reward_token,
        } => execute_update_config(deps, info, owner, staking_addr, reward_rate, reward_token),
        ExecuteMsg::Distribute {} => execute_distribute(deps, env),
    }
}

pub fn execute_update_config(
    deps: DepsMut,
    info: MessageInfo,
    owner: String,
    staking_addr: String,
    reward_rate: Uint128,
    reward_token: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if config.owner != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    let owner = deps.api.addr_validate(&owner)?;
    let staking_addr = deps.api.addr_validate(&staking_addr)?;
    if !validate_staking(deps.as_ref(), staking_addr.clone()) {
        return Err(ContractError::InvalidStakingContract {});
    }

    let reward_token = deps.api.addr_validate(&reward_token)?;
    if !validate_cw20(deps.as_ref(), reward_token.clone()) {
        return Err(ContractError::InvalidCw20 {});
    }

    let config = Config {
        owner,
        staking_addr,
        reward_token,
        reward_rate,
    };
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::default())
}

pub fn validate_cw20(deps: Deps, cw20_addr: Addr) -> bool {
    let response: Result<cw20::TokenInfoResponse, StdError> = deps
        .querier
        .query_wasm_smart(cw20_addr, &cw20::Cw20QueryMsg::TokenInfo {});
    response.is_ok()
}

pub fn validate_staking(deps: Deps, staking_addr: Addr) -> bool {
    let response: Result<stake_cw20::msg::TotalStakedAtHeightResponse, StdError> =
        deps.querier.query_wasm_smart(
            staking_addr,
            &stake_cw20::msg::QueryMsg::TotalStakedAtHeight { height: None },
        );
    response.is_ok()
}

pub fn execute_distribute(deps: DepsMut, env: Env) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let last_payment_block = LAST_PAYMENT_BLOCK.load(deps.storage)?;
    let block_diff = env.block.height - last_payment_block;
    let pending_rewards: Uint128 = config.reward_rate * Uint128::new(block_diff.into());

    if pending_rewards == Uint128::zero() {
        return Err(ContractError::NoPendingPayments {});
    }

    let balance_info: cw20::BalanceResponse = deps.querier.query_wasm_smart(
        config.reward_token.clone(),
        &cw20::Cw20QueryMsg::Balance {
            address: env.contract.address.to_string(),
        },
    )?;

    if balance_info.balance == Uint128::zero() {
        return Err(ContractError::OutOfFunds {});
    }

    let amount = min(balance_info.balance, pending_rewards);
    LAST_PAYMENT_BLOCK.save(deps.storage, &env.block.height)?;

    let msg = to_binary(&cw20::Cw20ExecuteMsg::Send {
        contract: config.staking_addr.clone().into_string(),
        amount,
        msg: to_binary(&stake_cw20::msg::ReceiveMsg::Fund {}).unwrap(),
    })?;
    let send_msg: CosmosMsg = WasmMsg::Execute {
        contract_addr: config.reward_token.into(),
        msg,
        funds: vec![],
    }
    .into();
    Ok(Response::default().add_message(send_msg))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Info {} => to_binary(&query_info(deps, env)?),
    }
}

fn query_info(deps: Deps, env: Env) -> StdResult<InfoResponse> {
    let config = CONFIG.load(deps.storage)?;
    let last_payment_block = LAST_PAYMENT_BLOCK.load(deps.storage)?;
    let balance_info: cw20::BalanceResponse = deps.querier.query_wasm_smart(
        config.reward_token.clone(),
        &cw20::Cw20QueryMsg::Balance {
            address: env.contract.address.to_string(),
        },
    )?;

    Ok(InfoResponse {
        config,
        last_payment_block,
        balance: balance_info.balance,
    })
}
