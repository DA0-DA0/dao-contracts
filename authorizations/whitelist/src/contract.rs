#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, CosmosMsg, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdResult,
};
use cw2::set_contract_version;
use cw_auth_middleware::msg::{IsAuthorizedResponse, QueryAuthMsg};

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg};
use crate::state::{AUTHORIZED, DAO};
use cw_auth_middleware::ContractError as AuthorizationError;

const CONTRACT_NAME: &str = "crates.io:whitelist";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    DAO.save(deps.storage, &msg.dao)?;
    Ok(Response::default().add_attribute("action", "instantiate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Allow { addr } => {
            if info.sender != DAO.load(deps.storage)? {
                return Err(AuthorizationError::Unauthorized {
                    reason: Some("Only the dao can add authorizations".to_string()),
                }
                .into());
            }
            AUTHORIZED.save(deps.storage, addr, &Empty {})?;
            Ok(Response::default().add_attribute("action", "allow"))
        }
        ExecuteMsg::Remove { addr } => {
            if info.sender != DAO.load(deps.storage)? {
                return Err(AuthorizationError::Unauthorized {
                    reason: Some("Only the dao can remove authorizations".to_string()),
                }
                .into());
            }
            AUTHORIZED.remove(deps.storage, addr);
            Ok(Response::default().add_attribute("action", "remove"))
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryAuthMsg) -> StdResult<Binary> {
    println!("WHITELIST QUERY");
    match msg {
        QueryAuthMsg::Authorize { msgs, sender } => authorize_messages(deps, env, msgs, sender),
        _ => unimplemented!(),
    }
}

fn authorize_messages(
    deps: Deps,
    _env: Env,
    _msgs: Vec<CosmosMsg<Empty>>,
    sender: String,
) -> StdResult<Binary> {
    // This checks all the registered authorizations
    let authorized = AUTHORIZED.may_load(deps.storage, sender)?.is_some();
    to_binary(&IsAuthorizedResponse { authorized })
}

#[cfg(test)]
mod tests {}
