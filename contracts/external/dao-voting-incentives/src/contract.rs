#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Uint128,
};
use cw2::set_contract_version;
use cw_ownable::get_ownership;
use cw_utils::Expiration;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use crate::state::{Config, CONFIG};
use crate::{execute, query};

pub(crate) const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
pub(crate) const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Save ownership
    let ownership = cw_ownable::initialize_owner(deps.storage, deps.api, Some(&msg.owner))?;

    // Validate denom
    let denom = msg.denom.into_checked(deps.as_ref())?;

    // Validate expiration
    if msg.expiration.is_expired(&env.block) {
        return Err(ContractError::AlreadyExpired {});
    }
    if let Expiration::Never {} = msg.expiration {
        return Err(ContractError::NotExpired {
            expiration: Expiration::Never {},
        });
    }

    // Save voting incentives config
    CONFIG.save(
        deps.storage,
        &Config {
            start_height: env.block.height,
            expiration: msg.expiration,
            denom: denom.clone(),
            total_votes: Uint128::zero(),
            expiration_balance: None,
        },
    )?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("creator", info.sender)
        .add_attribute("expiration", msg.expiration.to_string())
        .add_attribute("denom", denom.to_string())
        .add_attributes(ownership.into_attributes()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Claim {} => execute::claim(deps, env, info),
        ExecuteMsg::VoteHook(msg) => execute::vote_hook(deps, env, info, msg),
        ExecuteMsg::Expire {} => execute::expire(deps, env, info),
        ExecuteMsg::UpdateOwnership(action) => execute::update_ownership(deps, env, info, action),
        ExecuteMsg::Receive(cw20_receive_msg) => {
            execute::receive_cw20(deps, env, info, cw20_receive_msg)
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Rewards { address } => to_json_binary(&query::rewards(deps, env, address)?),
        QueryMsg::Config {} => to_json_binary(&query::config(deps)?),
        QueryMsg::Ownership {} => to_json_binary(&get_ownership(deps.storage)?),
        QueryMsg::Votes { address } => to_json_binary(&query::votes(deps, address)?),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    // Set contract to version to latest
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}
