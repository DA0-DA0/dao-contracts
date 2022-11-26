use cosmwasm_std::{
    from_binary,
    testing::{mock_dependencies, mock_env, mock_info},
    to_binary, Addr, Binary, ContractResult, Empty, Response, SubMsg, WasmMsg,
};
use cw_hooks::HooksResponse;
use dao_voting::status::Status;

use crate::{
    error::PreProposeError,
    msg::{ExecuteMsg, QueryMsg},
    state::{Config, PreProposeContract},
};

type Contract = PreProposeContract<Empty, Empty, Empty, Empty>;

#[test]
fn test_completed_hook_status_invariant() {
    let mut deps = mock_dependencies();
    let info = mock_info("pm", &[]);

    let module = Contract::default();

    module
        .proposal_module
        .save(&mut deps.storage, &Addr::unchecked("pm"))
        .unwrap();

    let res = module.execute(
        deps.as_mut(),
        mock_env(),
        info,
        ExecuteMsg::ProposalCompletedHook {
            proposal_id: 1,
            new_status: Status::Passed,
        },
    );

    assert_eq!(
        res.unwrap_err(),
        PreProposeError::NotClosedOrExecuted {
            status: Status::Passed
        }
    );
}

#[test]
fn test_completed_hook_auth() {
    let mut deps = mock_dependencies();
    let info = mock_info("evil", &[]);
    let module = Contract::default();

    module
        .proposal_module
        .save(&mut deps.storage, &Addr::unchecked("pm"))
        .unwrap();

    let res = module.execute(
        deps.as_mut(),
        mock_env(),
        info,
        ExecuteMsg::ProposalCompletedHook {
            proposal_id: 1,
            new_status: Status::Passed,
        },
    );

    assert_eq!(res.unwrap_err(), PreProposeError::NotModule {});
}

#[test]
fn test_proposal_submitted_hooks() {
    let mut deps = mock_dependencies();
    let module = Contract::default();

    module
        .dao
        .save(&mut deps.storage, &Addr::unchecked("d"))
        .unwrap();
    module
        .proposal_module
        .save(&mut deps.storage, &Addr::unchecked("pm"))
        .unwrap();
    module
        .config
        .save(
            &mut deps.storage,
            &Config {
                deposit_info: None,
                open_proposal_submission: true,
            },
        )
        .unwrap();

    // The DAO can add a hook.
    let info = mock_info("d", &[]);
    module
        .execute_add_proposal_submitted_hook(deps.as_mut(), info, "one".to_string())
        .unwrap();
    let hooks: HooksResponse = from_binary(
        &module
            .query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::ProposalSubmittedHooks {},
            )
            .unwrap(),
    )
    .unwrap();
    assert_eq!(hooks.hooks, vec!["one".to_string()]);

    // Non-DAO addresses can not add hooks.
    let info = mock_info("n", &[]);
    let err = module
        .execute_add_proposal_submitted_hook(deps.as_mut(), info, "two".to_string())
        .unwrap_err();
    assert_eq!(err, PreProposeError::NotDao {});

    deps.querier.update_wasm(|_| {
        // for responding to the next proposal ID query that gets fired by propose.
        cosmwasm_std::SystemResult::Ok(ContractResult::Ok(to_binary(&1u64).unwrap()))
    });

    // The hooks fire when a proposal is created.
    let res = module
        .execute(
            deps.as_mut(),
            mock_env(),
            mock_info("a", &[]),
            ExecuteMsg::Propose {
                msg: Empty::default(),
            },
        )
        .unwrap();
    assert_eq!(
        res.messages[1],
        SubMsg::new(WasmMsg::Execute {
            contract_addr: "one".to_string(),
            msg: to_binary(&Empty::default()).unwrap(),
            funds: vec![],
        })
    );

    // Non-DAO addresses can not remove hooks.
    let info = mock_info("n", &[]);
    let err = module
        .execute_remove_proposal_submitted_hook(deps.as_mut(), info, "one".to_string())
        .unwrap_err();
    assert_eq!(err, PreProposeError::NotDao {});

    // The DAO can remove a hook.
    let info = mock_info("d", &[]);
    module
        .execute_remove_proposal_submitted_hook(deps.as_mut(), info, "one".to_string())
        .unwrap();
    let hooks: HooksResponse = from_binary(
        &module
            .query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::ProposalSubmittedHooks {},
            )
            .unwrap(),
    )
    .unwrap();
    assert!(hooks.hooks.is_empty());
}

#[test]
fn test_query_ext_does_nothing() {
    let deps = mock_dependencies();
    let module = Contract::default();

    let res = module
        .query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::QueryExtension {
                msg: Empty::default(),
            },
        )
        .unwrap();
    assert_eq!(res, Binary::default())
}

#[test]
fn test_execute_ext_does_nothing() {
    let mut deps = mock_dependencies();
    let module = Contract::default();

    let res = module
        .execute(
            deps.as_mut(),
            mock_env(),
            mock_info("addr", &[]),
            ExecuteMsg::Extension {
                msg: Empty::default(),
            },
        )
        .unwrap();
    assert_eq!(res, Response::default())
}
