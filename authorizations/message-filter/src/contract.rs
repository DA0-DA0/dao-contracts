use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, CosmosMsg, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdResult,
};
use cw2::set_contract_version;
use cw_auth_middleware::msg::{IsAuthorizedResponse, QueryMsg};
use schemars::_serde_json::{json, Value};
#[cfg(not(feature = "library"))]
use std::ops::Deref;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg};
use crate::state::{ALLOWED, DAO};
use cw_auth_middleware::ContractError as AuthorizationError;

const CONTRACT_NAME: &str = "crates.io:whitelist";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

fn from_msg(msg: &CosmosMsg) -> Result<Value, ContractError> {
    serde_json::to_value(&msg).map_err(|_| ContractError::CustomError {
        val: "invalid CosmosMsg".to_string(),
    })
}

fn from_str(msg: &str) -> Result<Value, ContractError> {
    serde_json::from_str(msg).map_err(|_| ContractError::CustomError {
        val: "Invalid str".to_string(),
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
        ExecuteMsg::AllowMessages { msgs } => {
            if info.sender != DAO.load(deps.storage)? {
                return Err(AuthorizationError::Unauthorized {
                    reason: Some("Only the dao can add authorizations".to_string()),
                }
                .into());
            }

            println!(
                "{:?}",
                deep_partial_match(&from_msg(&msgs[0])?, &json!({"bank": {}}).into())
            );

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
            deep_partial_match(&from_msg(&msg).unwrap(), &json!({"bank": {}}).into()),
            true,
        );

        // Non-matching messages should fail
        assert_eq!(
            deep_partial_match(
                &from_str(r#"{"test": 1}"#).unwrap(),
                &json!({"bank": {}}).into()
            ),
            false,
        );

        // Partial messages work
        assert_eq!(
            deep_partial_match(
                &from_str(r#"{"bank": [1,2,3]}"#).unwrap(),
                &json!({"bank": {}}).into()
            ),
            true
        );

        // Testing array comparison as a proxy for all other Eq for Values
        assert_eq!(
            deep_partial_match(
                &json!({"bank": [1,3,2]}).into(),
                &from_str(r#"{"bank": [1,2,3]}"#).unwrap(),
            ),
            false
        );
        assert_eq!(
            deep_partial_match(
                &json!({"bank": [1,2,3]}).into(),
                &from_str(r#"{"bank": [1,2,3]}"#).unwrap(),
            ),
            true
        );

        // The partial json comparison only works in one direction
        assert_eq!(
            deep_partial_match(
                &json!({"bank": {}}).into(),
                &from_str(r#"{"bank": [1,2,3]}"#).unwrap()
            ),
            false
        );
    }
}
