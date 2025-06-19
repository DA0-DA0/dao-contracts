use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::{coins, from_json};

use crate::contract::{execute, instantiate, query};
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies();

    let msg = InstantiateMsg {
        owner: None,
        enabled: Some(true),
        initial_roles: None,
    };
    let info = mock_info("creator", &coins(1000, "earth"));

    // we can just call .unwrap() to assert this was a success
    let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());
}

#[test]
fn test_set_enabled() {
    let mut deps = mock_dependencies();

    // Instantiate contract
    let msg = InstantiateMsg {
        owner: None,
        enabled: Some(true),
        initial_roles: None,
    };
    let info = mock_info("creator", &[]);
    instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    // Disable the system
    let msg = ExecuteMsg::SetEnabled { enabled: false };
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!("set_enabled", res.attributes[0].value);

    // Query enabled state
    let res = query(deps.as_ref(), mock_env(), QueryMsg::IsEnabled {}).unwrap();
    let value: crate::msg::IsEnabledResponse = from_json(&res).unwrap();
    assert!(!value.enabled);
}
