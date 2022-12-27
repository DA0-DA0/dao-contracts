use crate::contract::{execute, execute_delegate, execute_execute, instantiate};
use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg};
use crate::state::Delegation;
use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::Addr;
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
    )
    .unwrap();

    let info = mock_info("not_admin", &[]);
    let env = mock_env();
    let err = execute_delegate(
        deps.as_mut(),
        env,
        info,
        Delegation {
            delegate: Addr::unchecked("dest_addr"),
            msgs: Vec::new(),
            expiration: None,
            policy_irrevocable: false,
            policy_preserve_on_failure: true,
        },
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::Unauthorized {}));

    // Admin delegation succeeds
    let env = mock_env();
    let info = mock_info(ADMIN_ADDR, &[]);
    execute_delegate(
        deps.as_mut(),
        env,
        info,
        Delegation {
            delegate: Addr::unchecked("dest_addr"),
            msgs: Vec::new(),
            expiration: None,
            policy_irrevocable: false,
            policy_preserve_on_failure: false,
        },
    )
    .unwrap();
}

// Only delegated can execute
// Admin cannot execute
// Non-admin non-delegated cannot execute
#[test]
fn test_execute_authorization() {
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
    )
    .unwrap();
    let env = mock_env();
    let info = mock_info(ADMIN_ADDR, &[]);
    let res = execute_delegate(
        deps.as_mut(),
        env,
        info,
        Delegation {
            delegate: Addr::unchecked("dest_addr"),
            msgs: Vec::new(),
            expiration: None,
            policy_irrevocable: false,
            policy_preserve_on_failure: false,
        },
    )
    .unwrap();

    let delegate_id_attr = &res
        .attributes
        .iter()
        .find(|&attr| attr.key == "delegate_id")
        .unwrap();
    let delegate_id = delegate_id_attr.value.parse::<u64>().unwrap();
    assert_eq!(delegate_id, 1);

    // Non-admin cannot execute
    let err = execute_execute(
        deps.as_mut(),
        mock_env(),
        mock_info("not an admin", &[]),
        delegate_id,
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::Unauthorized {}));

    // Admin cannot execute
    let err = execute_execute(
        deps.as_mut(),
        mock_env(),
        mock_info(ADMIN_ADDR, &[]),
        delegate_id,
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::Unauthorized {}));

    // Delegated address can execute
    execute_execute(
        deps.as_mut(),
        mock_env(),
        mock_info("dest_addr", &[]),
        delegate_id,
    )
    .unwrap();

    // - Second delegation independent from the first
    let res = execute_delegate(
        deps.as_mut(),
        mock_env(),
        mock_info(ADMIN_ADDR, &[]),
        Delegation {
            delegate: Addr::unchecked("dest_addr_2"),
            msgs: Vec::new(),
            expiration: None,
            policy_irrevocable: false,
            policy_preserve_on_failure: false,
        },
    )
    .unwrap();
    let delegate_id_attr = &res
        .attributes
        .iter()
        .find(|&attr| attr.key == "delegate_id")
        .unwrap();
    let delegate_id = delegate_id_attr.value.parse::<u64>().unwrap();
    assert_eq!(delegate_id, 2);

    // Previously Delegated address cannot execute
    let err = execute_execute(
        deps.as_mut(),
        mock_env(),
        mock_info("dest_addr", &[]),
        delegate_id,
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::Unauthorized {}));

    // New delegate executes new delegation
    execute_execute(
        deps.as_mut(),
        mock_env(),
        mock_info("dest_addr_2", &[]),
        delegate_id,
    )
    .unwrap();

    // New delegate cannot execute previous delegation
    let err =
        execute_execute(deps.as_mut(), mock_env(), mock_info("dest_addr_2", &[]), 1).unwrap_err();
    assert!(matches!(err, ContractError::Unauthorized {}));
}

// Can only execute once for false `preserve_on_failure`
// Can execute multiple if `preserve_on_failure` is true
// and execution fails.
#[test]
fn test_execute_on_failure_policy() {}

// Cannot execute delegation if expired
#[test]
fn test_execute_on_expired() {}

// Un-authorized revocation should fail
// If delegation is irrevocable, admin cannot revoke
#[test]
fn test_revocable_policy() {}
