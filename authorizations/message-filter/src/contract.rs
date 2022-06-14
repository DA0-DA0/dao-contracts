use cosmwasm_std::{entry_point, Uint128};
use cosmwasm_std::{Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw2::set_contract_version;
use schemars::{JsonSchema, Map};
use serde_derive::{Deserialize, Serialize};
use serde_json_wasm::{from_str, to_string};

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{Authorization, Config, Kind, ALLOWED, CONFIG};
use cw_auth_middleware::ContractError as AuthorizationError;

const CONTRACT_NAME: &str = "crates.io:whitelist";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[serde(untagged)]
pub enum Value {
    Null,
    Bool(bool),
    U128(Uint128),
    Number(u64), // Why doesn't u128 work here?
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
    let config = Config {
        dao: msg.dao,
        kind: msg.kind,
    };
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::default().add_attribute("action", "instantiate"))
}

fn deep_partial_match(msg: &Value, authorization: &Value) -> bool {
    match authorization {
        Value::Object(auth_map) => {
            if auth_map.is_empty() {
                return true;
            }

            let mut matching = true;
            if let Value::Object(msg_map) = msg {
                for (key, val) in auth_map {
                    if !msg_map.contains_key(key) {
                        return false;
                    };
                    matching = matching & deep_partial_match(msg_map.get(key).unwrap(), val);
                }
            } else {
                return false;
            }
            matching
        }
        Value::Array(auth_array) => {
            // Comparing arrays manually because PartialEq doesn't understand use our deep matching.
            let mut matching = true;
            if let Value::Array(msg_array) = msg {
                if msg_array.len() != auth_array.len() {
                    return false;
                }
                for (i, elem) in auth_array.iter().enumerate() {
                    matching = matching & deep_partial_match(&msg_array[i], &elem);
                }
            } else {
                return false;
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
        ExecuteMsg::AddAuthorization { addr, msg } => {
            let config = CONFIG.load(deps.storage)?;
            if info.sender != config.dao {
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
                    let new_auth = Authorization { addr, matcher: msg };
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
        ExecuteMsg::RemoveAuthorization { addr, msg } => {
            let config = CONFIG.load(deps.storage)?;
            if info.sender != config.dao {
                return Err(AuthorizationError::Unauthorized {
                    reason: Some("Only the dao can add authorizations".to_string()),
                }
                .into());
            }

            ALLOWED.update(
                deps.storage,
                addr.clone(),
                |auth: Option<Vec<Authorization>>| -> Result<Vec<Authorization>, ContractError> {
                    match auth {
                        Some(mut auth) => {
                            let i = auth.iter().position(|x| *x.matcher == msg);
                            if i.is_none() {
                                return Err(ContractError::NotFound {});
                            }
                            auth.remove(i.unwrap());
                            Ok(auth)
                        }
                        None => Err(ContractError::NotFound {}),
                    }
                },
            )?;
            Ok(Response::default().add_attribute("action", "removed"))
        }
        ExecuteMsg::Authorize { msgs, sender } => {
            let config = CONFIG.load(deps.storage)?;
            let auths = ALLOWED.load(deps.storage, sender);

            // If there are no auths, return the default for each Kind
            if auths.is_err() {
                return config.default_response();
            }

            let auths = auths.unwrap();

            // check that all messages can be converted to values
            for m in &msgs {
                msg_to_value(&m)?;
            }
            // check that all auths can be converted to values
            for a in &auths {
                str_to_value(&a.matcher)?;
            }

            // TODO: Do this manually instead of using any/all so we can provide better error messages
            let matched = auths.iter().any(|a| {
                msgs.iter().all(|m| {
                    deep_partial_match(
                        &msg_to_value(&m).unwrap(),
                        &str_to_value(&a.matcher).unwrap(),
                    )
                })
            });

            if matched {
                return match config.kind {
                    Kind::Allow {} => Ok(Response::default().add_attribute("allowed", "true")),
                    Kind::Reject {} => Err(AuthorizationError::Unauthorized {
                        reason: Some("Rejected by auth".to_string()),
                    }
                    .into()),
                };
            }
            config.default_response()
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        _ => unimplemented!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::{coins, BankMsg};

    #[test]
    fn test_deep_partial_match_simple() {
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

        // The partial json comparison works with any json
        assert_eq!(
            deep_partial_match(
                &from_str(r#"{"send": {"to_address": {}}}"#).unwrap(),
                &from_str(r#"{"send": {"to_address": "test"}}"#).unwrap()
            ),
            false
        );
    }

    #[test]
    fn test_deep_partial_match_complex() {
        let to_address = String::from("an_address");
        let amount = coins(1015, "earth");
        let bank = BankMsg::Send {
            to_address: to_address.clone(),
            amount,
        };
        let msg: CosmosMsg = bank.clone().into();

        assert_eq!(
            deep_partial_match(
                &msg_to_value(&msg).unwrap(),
                &from_str(r#"{"bank": {"send": {"to_address": "an_address", "amount": {}}}}"#)
                    .unwrap(),
            ),
            true
        );

        // Changing amouont
        let amount = coins(1234, "juno");
        let bank = BankMsg::Send {
            to_address: to_address.clone(),
            amount,
        };
        let msg: CosmosMsg = bank.clone().into();

        assert_eq!(
            deep_partial_match(
                &msg_to_value(&msg).unwrap(),
                &from_str(r#"{"bank": {"send": {"to_address": "an_address", "amount": {}}}}"#)
                    .unwrap(),
            ),
            true
        );

        // Changing address
        let amount = coins(1234, "juno");
        let bank = BankMsg::Send {
            to_address: "other_addr".to_string(),
            amount,
        };
        let msg: CosmosMsg = bank.clone().into();

        assert_eq!(
            deep_partial_match(
                &msg_to_value(&msg).unwrap(),
                &from_str(r#"{"bank": {"send": {"to_address": "an_address", "amount": {}}}}"#)
                    .unwrap(),
            ),
            false
        );
    }
}
