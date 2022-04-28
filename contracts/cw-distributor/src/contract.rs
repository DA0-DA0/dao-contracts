#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{ Uint128, CosmosMsg, WasmMsg, to_binary, Coin };

use cosmwasm_std::{
    Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
};
use cw20::Denom::Cw20;
use cw20::{Denom};
use cw2::set_contract_version;
use stake_cw20_external_rewards::msg::{ ReceiveMsg, ExecuteMsg::Fund };


use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, ConfigResponse};
use crate::state::{CONFIG, Config, LAST_PAYMENT_BLOCK};

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


    let token = match msg.token {
        Denom::Native(denom) => Denom::Native(denom),
        Cw20(addr) => Cw20(deps.api.addr_validate(&addr.to_string())?),
    };

    let config = Config {
        owner,
        recipient,
        token,
        reward_rate: msg.reward_rate,
    };
    CONFIG.save(deps.storage, &config)?;

    LAST_PAYMENT_BLOCK.save(deps.storage, &env.block.height)?;

    Ok(Response::new()
        .add_attribute("action", "instantiate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateConfig { owner, recipient, reward_rate, token } => {
            execute_update_config(deps, info, owner, recipient, reward_rate, token)
        },
        ExecuteMsg::Distribute {} => {
            execute_distribute(deps, env)
        }
    }
}

pub fn execute_update_config (
    deps: DepsMut,
    info: MessageInfo,
    owner: String,
    recipient: String,
    reward_rate: Uint128,
    token: Denom
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if config.owner != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    let owner = deps.api.addr_validate(&owner)?;
    let recipient = deps.api.addr_validate(&recipient)?;

    let token = match token {
        Denom::Native(denom) => Denom::Native(denom),
        Cw20(addr) => Cw20(deps.api.addr_validate(&addr.to_string())?),
    };

    let config = Config {
        owner,
        recipient,
        token,
        reward_rate,
    };
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::default())
}

pub fn execute_distribute (
    deps: DepsMut,
    env: Env,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let last_payment_block = LAST_PAYMENT_BLOCK.load(deps.storage)?;
    let block_diff = env.block.height - last_payment_block;
    let amount: Uint128 = config.reward_rate * Uint128::new(block_diff.into());

    if amount == Uint128::zero() {
        return Err(ContractError::NoPendingPayments {  });
    }


    LAST_PAYMENT_BLOCK.save(deps.storage, &env.block.height)?;

   let fund_msg: CosmosMsg = match config.token {
        Denom::Native(denom) => {
            WasmMsg::Execute {
                contract_addr: config.recipient.into_string(),
                msg: to_binary(&Fund {})?,
                funds:vec![Coin { denom, amount }],
            }.into()
        },
        Denom::Cw20(addr) => {
            let cw20_msg = to_binary(&cw20::Cw20ExecuteMsg::Send {
                contract: config.recipient.into_string(),
                amount,
                msg: to_binary(&ReceiveMsg::Fund{}).unwrap()
            })?;
            WasmMsg::Execute {
                contract_addr: addr.into_string(),
                msg: cw20_msg,
                funds: vec![],
            }
            .into()
        }
    };

    Ok(Response::default().add_message(fund_msg))
}


#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse { config })
}
