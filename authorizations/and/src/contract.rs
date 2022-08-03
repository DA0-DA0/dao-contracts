#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, wasm_execute, Addr, Binary, CosmosMsg, Deps, DepsMut, Empty, Env, MessageInfo,
    Order, Response, StdResult, SubMsg,
};
use cw2::set_contract_version;
use cw_auth_middleware::msg::{IsAuthorizedResponse, QueryMsg};

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg};
use crate::state::{CHILDREN, PARENT};
use cw_auth_middleware::ContractError as AuthorizationError;

const CONTRACT_NAME: &str = "crates.io:whitelist";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const UPDATE_REPLY_ID: u64 = 1_000_000;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    PARENT.save(deps.storage, &msg.parent)?;
    for child in msg.children {
        CHILDREN.save(deps.storage, child, &cosmwasm_std::Empty {})?;
    }
    Ok(Response::default().add_attribute("action", "instantiate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    if info.sender != PARENT.load(deps.storage)? {
        return Err(AuthorizationError::Unauthorized {
            reason: Some("Only the parent can execute on this contract".to_string()),
        }
        .into());
    }

    match msg {
        ExecuteMsg::AddChild { addr } => {
            CHILDREN.save(deps.storage, addr, &Empty {})?;
            Ok(Response::default().add_attribute("action", "allow"))
        }
        ExecuteMsg::RemoveChild { addr } => {
            CHILDREN.remove(deps.storage, addr);
            Ok(Response::default().add_attribute("action", "remove"))
        }
        ExecuteMsg::UpdateExecutedAuthorizationState { msgs, sender } => {
            // This authorization is a passthrough, so any update messages
            let response = Response::default()
                .add_attribute("action", "execute_authorize")
                .add_attribute("authorized", "true");

            CHILDREN
                .range(deps.storage, None, None, Order::Ascending)
                .fold(
                    Ok(response),
                    |acc, addr| -> Result<Response, ContractError> {
                        // All errors from submessages are ignored since the validation should already have been done by the parent.
                        // This assumes the parent handles the authorizations spec
                        Ok(acc?.add_submessage(SubMsg::reply_on_error(
                            wasm_execute(
                                addr?.0.to_string(),
                                &ExecuteMsg::UpdateExecutedAuthorizationState {
                                    msgs: msgs.clone(),
                                    sender: sender.clone(),
                                },
                                vec![],
                            )?,
                            UPDATE_REPLY_ID,
                        )))
                    },
                )
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Authorize { msgs, sender } => authorize_messages(deps, env, msgs, sender),
        _ => unimplemented!(),
    }
}

fn authorize_messages(
    deps: Deps,
    _env: Env,
    msgs: Vec<CosmosMsg>,
    sender: Addr,
) -> StdResult<Binary> {
    let children: Result<Vec<_>, _> = CHILDREN
        .range(deps.storage, None, None, Order::Ascending)
        .collect();
    if children.is_err() {
        return to_binary(&IsAuthorizedResponse { authorized: false });
    }
    let children = children.unwrap();

    // This checks all the registered authorizations return true
    let authorized = children.into_iter().map(|c| c.0).all(|a| {
        deps.querier
            .query_wasm_smart(
                a.clone(),
                &QueryMsg::Authorize {
                    msgs: msgs.clone(),
                    sender: sender.clone(),
                },
            )
            .unwrap_or(IsAuthorizedResponse { authorized: false })
            .authorized
    });

    to_binary(&IsAuthorizedResponse { authorized })
}

#[cfg(test)]
mod tests {}
