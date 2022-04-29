use std::cmp::min;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, CosmosMsg, Uint128, WasmMsg};

use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw2::set_contract_version;
use stake_cw20::msg::ReceiveMsg;

use crate::error::ContractError;
use crate::msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, LastPaymentBlockResponse, QueryMsg};
use crate::state::{Config, CONFIG, LAST_PAYMENT_BLOCK};

const CONTRACT_NAME: &str = "crates.io:cw-distributor";
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
    let recipient = deps.api.addr_validate(&msg.recipient)?;

    let reward_token = deps.api.addr_validate(&msg.reward_token)?;
    let config = Config {
        owner,
        recipient,
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
            recipient,
            reward_rate,
            reward_token,
        } => execute_update_config(deps, info, owner, recipient, reward_rate, reward_token),
        ExecuteMsg::Distribute {} => execute_distribute(deps, env),
    }
}

pub fn execute_update_config(
    deps: DepsMut,
    info: MessageInfo,
    owner: String,
    recipient: String,
    reward_rate: Uint128,
    reward_token: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if config.owner != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    let owner = deps.api.addr_validate(&owner)?;
    let recipient = deps.api.addr_validate(&recipient)?;
    let reward_token = deps.api.addr_validate(&reward_token)?;

    let config = Config {
        owner,
        recipient,
        reward_token,
        reward_rate,
    };
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::default())
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
    })?;

    if balance_info.balance == Uint128::zero() {
        return Err(ContractError::OutOfFunds {});
    }

    let amount = min(balance_info.balance, pending_rewards);
    LAST_PAYMENT_BLOCK.save(deps.storage, &env.block.height)?;

    let msg = to_binary(&cw20::Cw20ExecuteMsg::Send {
        contract: config.recipient.clone().into_string(),
        amount,
        msg: to_binary(&ReceiveMsg::Fund {}).unwrap(),
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
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::LastPaymentBlock {} => to_binary(&query_last_payment_block(deps)?),
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse { config })
}

fn query_last_payment_block(deps: Deps) -> StdResult<LastPaymentBlockResponse> {
    let last_payment_block = LAST_PAYMENT_BLOCK.load(deps.storage)?;
    Ok(LastPaymentBlockResponse { last_payment_block })
}
