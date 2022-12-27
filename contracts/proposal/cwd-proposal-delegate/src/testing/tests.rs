
use crate::contract::{execute, execute_delegate, instantiate};
use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg};
use crate::state::{Delegation};
use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::{Addr};
use std::matches;


const ADMIN_ADDR: &str = "admin";

// Non-admin cannot delegate a message
// Admin can delegate a message
#[test]
fn test_unauthorized_delegation() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info("any_addr", &[]);

    instantiate(
        deps.as_mut(),
        env,
        info,
        InstantiateMsg {
            admin: ADMIN_ADDR.to_string(),
        },
    ).unwrap();
  
    let info = mock_info("not_admin", &[]);
    let env = mock_env();
    let err = execute_delegate(deps.as_mut(), env, info, Delegation {
        delegate: Addr::unchecked("dest_addr"),
        msgs: Vec::new(),
        expiration: None,
        policy_irrevocable: false, 
        policy_preserve_on_failure: true 
    }).unwrap_err();
    assert!(matches!(err, ContractError::Unauthorized {}));

    // Admin delegation succeeds
    let env = mock_env();
    let info = mock_info(ADMIN_ADDR, &[]);
    execute_delegate(deps.as_mut(), env, info, Delegation {
        delegate: Addr::unchecked("dest_addr"),
        msgs: Vec::new(),
        expiration: None,
        policy_irrevocable: false, 
        policy_preserve_on_failure: true 
    }).unwrap();
}

// Only delegated can execute
// Admin cannot execute
// Non-admin non-delegated cannot execute
#[test]
fn test_execute_authorization() {}

// Can only execute once for policy
// Can execute multiple if `preserve_on_failure` is true
// and execution fails.
#[test]
fn test_execute_on_failure_policy() {}

// Cannot execute if expired
#[test]
fn test_execute_on_expired() {}
