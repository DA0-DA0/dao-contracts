use crate::contract::{
    execute_delegate, execute_execute, execute_remove_delegation, instantiate, reply,
    REPLY_ID_EXECUTE_PROPOSAL_HOOK,
};
use crate::error::ContractError;
use crate::msg::InstantiateMsg;
use crate::state::{Delegation, EXECUTE_CTX};
use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::{Addr, Reply, SubMsgResponse, SubMsgResult};
use cw_utils::Expiration;
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
            policy_module_irrevocable: false,
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
            policy_module_irrevocable: false,
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
            policy_module_irrevocable: false,
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
            policy_module_irrevocable: false,
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
fn test_execute_on_failure_policy() {
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
    execute_delegate(
        deps.as_mut(),
        env,
        info,
        Delegation {
            delegate: Addr::unchecked("dest_addr"),
            msgs: Vec::new(),
            expiration: None,
            policy_module_irrevocable: false,
            policy_preserve_on_failure: false,
        },
    )
    .unwrap();

    // Multiple execution fails
    let failed_reply_msg = Reply {
        id: REPLY_ID_EXECUTE_PROPOSAL_HOOK,
        result: SubMsgResult::Err("Execution of delegated message failed".to_string()),
    };
    EXECUTE_CTX.save(deps.as_mut().storage, &1).unwrap();
    reply(deps.as_mut(), mock_env(), failed_reply_msg).unwrap();

    let err =
        execute_execute(deps.as_mut(), mock_env(), mock_info("dest_addr", &[]), 1).unwrap_err();
    assert!(matches!(err, ContractError::DelegationNotFound {}));

    // `preserve_on_failure` set to true
    {
        let delegate_id: u64 = 2;
        let info = mock_info(ADMIN_ADDR, &[]);
        execute_delegate(
            deps.as_mut(),
            mock_env(),
            info,
            Delegation {
                delegate: Addr::unchecked("dest_addr"),
                msgs: Vec::new(),
                expiration: None,
                policy_module_irrevocable: false,
                policy_preserve_on_failure: true,
            },
        )
        .unwrap();
        // For `preserve_on_failure`, multiple failed execution is ok
        let failed_reply_msg = Reply {
            id: REPLY_ID_EXECUTE_PROPOSAL_HOOK,
            result: SubMsgResult::Err("Execution of delegated message failed".to_string()),
        };
        EXECUTE_CTX
            .save(deps.as_mut().storage, &delegate_id)
            .unwrap();
        reply(deps.as_mut(), mock_env(), failed_reply_msg.clone()).unwrap();
        reply(deps.as_mut(), mock_env(), failed_reply_msg.clone()).unwrap();
        reply(deps.as_mut(), mock_env(), failed_reply_msg.clone()).unwrap();

        let ok_reply_msg = Reply {
            id: REPLY_ID_EXECUTE_PROPOSAL_HOOK,
            result: SubMsgResult::Ok(SubMsgResponse {
                events: Vec::new(),
                data: None,
            }),
        };
        reply(deps.as_mut(), mock_env(), ok_reply_msg).unwrap();

        // Successful execution prevents future execution
        let err = execute_execute(
            deps.as_mut(),
            mock_env(),
            mock_info("dest_addr", &[]),
            delegate_id,
        )
        .unwrap_err();
        assert!(matches!(err, ContractError::DelegationNotFound {}));
    }
}

// Cannot execute delegation if expired
#[test]
fn test_execute_on_expired() {
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

    let mut env = mock_env();
    env.block.height = 0;
    execute_delegate(
        deps.as_mut(),
        env,
        mock_info(ADMIN_ADDR, &[]),
        Delegation {
            delegate: Addr::unchecked("dest_addr"),
            msgs: Vec::new(),
            expiration: Some(Expiration::AtHeight(10)),
            policy_module_irrevocable: false,
            policy_preserve_on_failure: false,
        },
    )
    .unwrap();

    // 10 height should fail
    let mut expired_env = mock_env();
    expired_env.block.height = 10;
    let err =
        execute_execute(deps.as_mut(), expired_env, mock_info("dest_addr", &[]), 1).unwrap_err();
    assert!(matches!(err, ContractError::DelegationExpired {}));

    // 9 height should succeed
    let mut not_expired_env = mock_env();
    not_expired_env.block.height = 9;
    execute_execute(
        deps.as_mut(),
        not_expired_env,
        mock_info("dest_addr", &[]),
        1,
    )
    .unwrap();
}

// Un-authorized revocation should fail
// If delegation is irrevocable, admin cannot revoke
#[test]
fn test_revocable_policy() {
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
    execute_delegate(
        deps.as_mut(),
        env,
        info,
        Delegation {
            delegate: Addr::unchecked("dest_addr"),
            msgs: Vec::new(),
            expiration: None,
            policy_module_irrevocable: true, // Make it irrevocable
            policy_preserve_on_failure: false,
        },
    )
    .unwrap();

    // Admin and non-admin cannot revoke an irrevocable delegation
    let err = execute_remove_delegation(deps.as_mut(), mock_env(), mock_info(ADMIN_ADDR, &[]), 1)
        .unwrap_err();
    assert!(matches!(err, ContractError::DelegationIrrevocable {}));
    let err = execute_remove_delegation(deps.as_mut(), mock_env(), mock_info("non-admin", &[]), 1)
        .unwrap_err();
    assert!(matches!(err, ContractError::Unauthorized {}));

    // Make revocable delegation
    execute_delegate(
        deps.as_mut(),
        mock_env(),
        mock_info(ADMIN_ADDR, &[]),
        Delegation {
            delegate: Addr::unchecked("dest_addr"),
            msgs: Vec::new(),
            expiration: None,
            policy_module_irrevocable: false,
            policy_preserve_on_failure: false,
        },
    )
    .unwrap(); // has id of `2`
    let revocable_delegate_id: u64 = 2;

    // Non-admin cannot revoke a revocable delegation
    let err = execute_remove_delegation(
        deps.as_mut(),
        mock_env(),
        mock_info("non-admin", &[]),
        revocable_delegate_id,
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::Unauthorized {}));

    // Admin can revoke a revocable delegation
    execute_remove_delegation(
        deps.as_mut(),
        mock_env(),
        mock_info(ADMIN_ADDR, &[]),
        revocable_delegate_id,
    )
    .unwrap();

    // Can no longer execute
    let err = execute_execute(
        deps.as_mut(),
        mock_env(),
        mock_info("dest_addr", &[]),
        revocable_delegate_id,
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::DelegationNotFound {}));
}
