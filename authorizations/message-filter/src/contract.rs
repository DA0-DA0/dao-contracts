use cosmwasm_std::{entry_point, to_vec};
use cosmwasm_std::{
    Binary, CosmosMsg, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdResult,
};
use cw2::set_contract_version;
use cw_auth_middleware::msg::QueryMsg;
use schemars::{JsonSchema, Map};
use serde_derive::{Deserialize, Serialize};
use serde_json_wasm::{from_str, to_string};

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg};
use crate::state::{Authorization, Kind, ALLOWED, DAO};
use cw_auth_middleware::ContractError as AuthorizationError;

const CONTRACT_NAME: &str = "crates.io:whitelist";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[serde(untagged)]
pub enum Value {
    Null,
    Bool(bool),
    Number(u64),
    String(String),
    Array(Vec<Value>),
    Object(Map<String, Value>),
}

fn msg_to_value(msg: &CosmosMsg) -> Result<Value, ContractError> {
    let serialized = to_string(msg).map_err(|_| ContractError::CustomError {
        val: "invalid CosmosMsg".to_string(),
    })?;

    str_to_value(&serialized)
}

fn str_to_value(msg: &str) -> Result<Value, ContractError> {
    from_str(msg).map_err(|_| ContractError::CustomError {
        val: "invalid str".to_string(),
    })
}

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

fn deep_partial_match(msg: &Value, authorization: &Value) -> bool {
    match authorization {
        Value::Object(auth_map) => {
            let mut matching = true;
            for (key, val) in auth_map {
                if let Value::Object(msg_map) = msg {
                    if !msg_map.contains_key(key) {
                        return false;
                    };
                    match val {
                        Value::Object(internal) if internal.is_empty() => return matching,
                        _ => matching = deep_partial_match(msg_map.get(key).unwrap(), val),
                    }
                } else {
                    return false;
                }
            }
            matching
        }
        _ => authorization == msg,
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::AddAuthorization { kind, addr, msg } => {
            if info.sender != DAO.load(deps.storage)? {
                return Err(AuthorizationError::Unauthorized {
                    reason: Some("Only the dao can add authorizations".to_string()),
                }
                .into());
            }

            // If the message can't be converted to a string, we fail
            str_to_value(&msg)?;
            ALLOWED.update(
                deps.storage,
                addr.clone(),
                |auth: Option<Vec<Authorization>>| -> Result<Vec<Authorization>, ContractError> {
                    let new_auth = Authorization {
                        kind,
                        addr,
                        matcher: msg,
                    };
                    match auth {
                        Some(mut auth) => {
                            auth.push(new_auth);
                            Ok(auth)
                        }
                        None => Ok(vec![new_auth]),
                    }
                },
            )?;

            Ok(Response::default().add_attribute("action", "allow_message"))
        }
        ExecuteMsg::RemoveAuthorization { kind, addr, msg } => {
            unimplemented!()
        }
        ExecuteMsg::Authorize { msgs, sender } => {
            let auths = ALLOWED.load(deps.storage, sender).map_err(|_| {
                AuthorizationError::Unauthorized {
                    reason: Some("No authorizations for sender".to_string()),
                }
            })?;

            for msg in &msgs {
                for auth in &auths {
                    let check =
                        deep_partial_match(&msg_to_value(&msg)?, &str_to_value(&auth.matcher)?);
                    match auth.kind {
                        Kind::Allow {} => {
                            if check {
                                return Ok(Response::default().add_attribute("allowed", "true"));
                            }
                        }
                        Kind::Reject {} => {
                            if !check {
                                return Err(AuthorizationError::Unauthorized {
                                    reason: Some(format!("Test failed: {}", auth.matcher)),
                                }
                                .into());
                            }
                        }
                    }
                }
            }
            Err(AuthorizationError::Unauthorized {
                reason: Some("Not explicitly authorized, defaulting to reject".to_string()),
            }
            .into())
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
mod tests {
    use super::*;
    use cosmwasm_std::{coins, BankMsg};

    #[test]
    fn test_deep_partial_match() {
        let to_address = String::from("you");
        let amount = coins(1015, "earth");
        let bank = BankMsg::Send { to_address, amount };
        let msg: CosmosMsg = bank.clone().into();

        // Comparing a cosmos message to partial json
        assert_eq!(
            deep_partial_match(
                &msg_to_value(&msg).unwrap(),
                &from_str(r#"{"bank": {}}"#).unwrap()
            ),
            true,
        );

        // Non-matching messages should fail
        assert_eq!(
            deep_partial_match(
                &from_str(r#"{"test": 1}"#).unwrap(),
                &from_str(r#"{"bank": {}}"#).unwrap()
            ),
            false,
        );

        // Partial messages work
        assert_eq!(
            deep_partial_match(
                &from_str(r#"{"bank": [1,2,3]}"#).unwrap(),
                &from_str(r#"{"bank": {}}"#).unwrap()
            ),
            true
        );

        // Testing array comparison as a proxy for all other Eq for Values
        assert_eq!(
            deep_partial_match(
                &from_str(r#"{"bank": [1,3,2]}"#).unwrap(),
                &from_str(r#"{"bank": [1,2,3]}"#).unwrap(),
            ),
            false
        );
        assert_eq!(
            deep_partial_match(
                &from_str(r#"{"bank": [1,2,3]}"#).unwrap(),
                &from_str(r#"{"bank": [1,2,3]}"#).unwrap(),
            ),
            true
        );

        // The partial json comparison only works in one direction
        assert_eq!(
            deep_partial_match(
                &from_str(r#"{"bank": {}}"#).unwrap(),
                &from_str(r#"{"bank": [1,2,3]}"#).unwrap()
            ),
            false
        );
    }
}
