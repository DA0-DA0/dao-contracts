#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use std::cmp::min;

use cosmwasm_std::{
    from_binary, to_binary, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdError,
    StdResult, Uint128,
};

use cw20::Cw20ReceiveMsg;

use crate::msg::{ExecuteMsg, GetConfigResponse, InstantiateMsg, QueryMsg, ReceiveMsg};
use crate::state::{Config, CONFIG, LAST_CLAIM};
use crate::ContractError;
use cw2::set_contract_version;

pub use cw20_base::enumerable::{query_all_accounts, query_all_allowances};

const CONTRACT_NAME: &str = "crates.io:stake_cw20_rewards";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response<Empty>, ContractError> {
    // Validate config
    let blocks = Uint128::from(msg.end_block - msg.start_block);
    let calculated_total = msg
        .payment_per_block
        .checked_mul(blocks)
        .map_err(StdError::overflow)?;
    if calculated_total != msg.total_payment {
        return Err(ContractError::ConfigInvalid {});
    };

    let config = Config {
        token_address: msg.token_address,
        staking_contract: msg.staking_contract,
        payment_per_block: msg.payment_per_block,
        total_payment: msg.total_payment,
        start_block: msg.start_block,
        end_block: msg.end_block,
        funded: false,
    };
    CONFIG.save(deps.storage, &config)?;
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response<Empty>, ContractError> {
    match msg {
        ExecuteMsg::Receive(msg) => execute_receive(deps, env, info, msg),
        ExecuteMsg::Claim {} => execute_claim(deps, env),
    }
}

pub fn execute_receive(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    wrapper: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.token_address {
        return Err(ContractError::InvalidToken {
            received: info.sender,
            expected: config.token_address,
        });
    }
    let msg: ReceiveMsg = from_binary(&wrapper.msg)?;
    match msg {
        ReceiveMsg::Fund {} => execute_fund(deps, env, wrapper.amount),
    }
}

pub fn execute_fund(deps: DepsMut, _env: Env, amount: Uint128) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    if config.funded {
        return Err(ContractError::AlreadyFunded {});
    };
    if config.total_payment != amount {
        return Err(ContractError::IncorrectFundingAmount {});
    };
    config.funded = true;
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new()
        .add_attribute("action", "funded")
        .add_attribute("amount", amount))
}

pub fn execute_claim(deps: DepsMut, env: Env) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let last_claim = LAST_CLAIM.load(deps.storage).unwrap_or(config.start_block);

    if env.block.height < config.start_block {
        return Err(ContractError::RewardsNotStarted {});
    };
    if last_claim >= config.end_block {
        return Err(ContractError::RewardsFinished {});
    };
    if last_claim == env.block.height {
        return Err(ContractError::RewardsAlreadyClaimed {});
    };
    if !config.funded {
        return Err(ContractError::RewardsNotFunded {});
    };

    let blocks = Uint128::from(min(&env.block.height, &config.end_block) - last_claim);
    let reward_to_disburse = blocks
        .checked_mul(config.payment_per_block)
        .map_err(StdError::overflow)?;

    let sub_msg = to_binary(&stake_cw20::msg::ReceiveMsg::Fund {})?;
    let payment_msg = cw20::Cw20ExecuteMsg::Send {
        contract: config.staking_contract.to_string(),
        amount: reward_to_disburse,
        msg: sub_msg,
    };

    let cosmos_msg = cosmwasm_std::WasmMsg::Execute {
        contract_addr: config.token_address.to_string(),
        msg: to_binary(&payment_msg)?,
        funds: vec![],
    };

    LAST_CLAIM.save(deps.storage, &min(env.block.height, config.end_block))?;

    Ok(Response::new()
        .add_message(cosmos_msg)
        .add_attribute("action", "claim")
        .add_attribute("amount", reward_to_disburse))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetConfig {} => to_binary(&query_config(deps)?),
    }
}

pub fn query_config(deps: Deps) -> StdResult<GetConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(GetConfigResponse {
        token_address: config.token_address,
        staking_contract: config.staking_contract,
        payment_per_block: config.payment_per_block,
        total_payment: config.total_payment,
        start_block: config.start_block,
        end_block: config.end_block,
        funded: config.funded,
    })
}

#[cfg(test)]
mod tests {}
