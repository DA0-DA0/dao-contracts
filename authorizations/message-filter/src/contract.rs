#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, CosmosMsg, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdResult,
};
use cw2::set_contract_version;
use cw_auth_middleware::msg::{IsAuthorizedResponse, QueryMsg};
use schemars::_serde_json::Value;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg};
use crate::state::{ALLOWED, DAO};
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

fn deep_value_match(msg: &Value, authorization: &Value) -> bool {
    println!("entering with {:?}, {:?}", msg, authorization);
    match authorization {
        Value::Null => match msg {
            Value::Null => true,
            _ => false,
        },
        Value::Bool(x) => match msg {
            Value::Bool(y) => x == y,
            _ => false,
        },
        Value::Number(x) => match msg {
            Value::Number(y) => x == y,
            _ => false,
        },
        Value::String(_) => todo!(),
        Value::Array(_) => todo!(),
        Value::Object(auth_map) => {
            let mut matching = true;
            for (key, val) in auth_map {
                if let Value::Object(msg_map) = msg {
                    if !msg_map.contains_key(key) {
                        return false;
                    };
                    match val {
                        Value::Object(internal) if internal.is_empty() => return matching,
                        _ => matching = deep_value_match(msg_map.get(key).unwrap(), val),
                    }
                } else {
                    return false;
                }
            }
            matching
        }
    }
}

fn compare(msg: &Value, authorization: &str) -> Result<bool, ContractError> {
    let authorization =
        serde_json::from_str(authorization).map_err(|_| ContractError::CustomError {
            val: "bad stored auth".to_string(),
        })?;

    Ok(deep_value_match(msg, &authorization))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::AllowMessages { msgs } => {
            if info.sender != DAO.load(deps.storage)? {
                return Err(AuthorizationError::Unauthorized {
                    reason: Some("Only the dao can add authorizations".to_string()),
                }
                .into());
            }

            //let msg_json = serde_json::to_string(&msgs[0])?;
            let msg_map =
                serde_json::to_value(&msgs[0]).map_err(|_| ContractError::CustomError {
                    val: "bad message".to_string(),
                })?;
            println!("{:?}", msg_map);
            println!("{:?}", compare(&msg_map, "{\"bank\": {\"x\": 1}}"));

            Ok(Response::default().add_attribute("action", "allow_messages"))
        }
        ExecuteMsg::DisallowMessages { msgs } => {
            unimplemented!()
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
    _msgs: Vec<CosmosMsg<Empty>>,
    sender: String,
) -> StdResult<Binary> {
    // This checks all the registered authorizations
    // let authorized = AUTHORIZED.may_load(deps.storage, sender)?.is_some();
    // to_binary(&IsAuthorizedResponse { authorized })
    unimplemented!()
}

#[cfg(test)]
mod tests {}
