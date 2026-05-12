use cosmwasm_std::{
    from_json,
    testing::{mock_dependencies, mock_env, message_info},
    to_json_binary, Addr, Binary, ContractResult, Empty, Response, SubMsg, WasmMsg,
};
use cw_hooks::HooksResponse;
use dao_voting::{pre_propose::PreProposeSubmissionPolicy, status::Status};

use crate::{
    error::PreProposeError,
    msg::{ExecuteMsg, QueryMsg},
    state::{Config, PreProposeContract},
};

type Contract = PreProposeContract<Empty, Empty, Empty, Empty, Empty>;

#[test]
fn test_completed_hook_status_invariant() {
    let mut deps = mock_dependencies();
    let info = message_info(&Addr::unchecked("cosmwasm1jme0pqgalzxclsmthg0tle8ysswwwxc7ymz2czmheps0y63h5a3surh005"), &[]);

    let module = Contract::default();

    module
        .proposal_module
        .save(&mut deps.storage, &Addr::unchecked("cosmwasm1jme0pqgalzxclsmthg0tle8ysswwwxc7ymz2czmheps0y63h5a3surh005"))
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
        PreProposeError::NotCompleted {
            status: Status::Passed
        }
    );
}

#[test]
fn test_completed_hook_auth() {
    let mut deps = mock_dependencies();
    let info = message_info(&Addr::unchecked("cosmwasm1khqlkthud445vaxzlhxy3nspksarklqrwc7qcv64mcqfnms033eshh58j0"), &[]);
    let module = Contract::default();

    module
        .proposal_module
        .save(&mut deps.storage, &Addr::unchecked("cosmwasm1jme0pqgalzxclsmthg0tle8ysswwwxc7ymz2czmheps0y63h5a3surh005"))
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
        .save(&mut deps.storage, &Addr::unchecked("cosmwasm1rzkruu6r7qtgjrz3p6fljdfxz95ancl4v4pkg2vrp7hsjd85lrjqwpk08g"))
        .unwrap();
    module
        .proposal_module
        .save(&mut deps.storage, &Addr::unchecked("cosmwasm1jme0pqgalzxclsmthg0tle8ysswwwxc7ymz2czmheps0y63h5a3surh005"))
        .unwrap();
    module
        .config
        .save(
            &mut deps.storage,
            &Config {
                deposit_info: None,
                submission_policy: PreProposeSubmissionPolicy::Anyone { denylist: vec![] },
            },
        )
        .unwrap();

    // The DAO can add a hook.
    let info = message_info(&Addr::unchecked("cosmwasm1rzkruu6r7qtgjrz3p6fljdfxz95ancl4v4pkg2vrp7hsjd85lrjqwpk08g"), &[]);
    module
        .execute_add_proposal_submitted_hook(deps.as_mut(), info, "one".to_string())
        .unwrap();
    let hooks: HooksResponse = from_json(
        module
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
    let info = message_info(&Addr::unchecked("cosmwasm1rvttrh6n3wsjmsle0mdmsh92wpgdgmq5sy6zjrlt4q8cydkg8kusr83amx"), &[]);
    let err = module
        .execute_add_proposal_submitted_hook(deps.as_mut(), info, "two".to_string())
        .unwrap_err();
    assert_eq!(err, PreProposeError::NotDao {});

    deps.querier.update_wasm(|_| {
        // for responding to the next proposal ID query that gets fired by propose.
        cosmwasm_std::SystemResult::Ok(ContractResult::Ok(to_json_binary(&1u64).unwrap()))
    });

    // The hooks fire when a proposal is created.
    let res = module
        .execute(
            deps.as_mut(),
            mock_env(),
            message_info(&Addr::unchecked("cosmwasm1e2tczyk2rw7u47kzxxee5g7ufkncdmlcz37yuu4espmcttlwfzasvcmsxa"), &[]),
            ExecuteMsg::Propose {
                msg: Empty::default(),
            },
        )
        .unwrap();
    assert_eq!(
        res.messages[1],
        SubMsg::new(WasmMsg::Execute {
            contract_addr: "one".to_string(),
            msg: to_json_binary(&Empty::default()).unwrap(),
            funds: vec![],
        })
    );

    // Non-DAO addresses can not remove hooks.
    let info = message_info(&Addr::unchecked("cosmwasm1rvttrh6n3wsjmsle0mdmsh92wpgdgmq5sy6zjrlt4q8cydkg8kusr83amx"), &[]);
    let err = module
        .execute_remove_proposal_submitted_hook(deps.as_mut(), info, "one".to_string())
        .unwrap_err();
    assert_eq!(err, PreProposeError::NotDao {});

    // The DAO can remove a hook.
    let info = message_info(&Addr::unchecked("cosmwasm1rzkruu6r7qtgjrz3p6fljdfxz95ancl4v4pkg2vrp7hsjd85lrjqwpk08g"), &[]);
    module
        .execute_remove_proposal_submitted_hook(deps.as_mut(), info, "one".to_string())
        .unwrap();
    let hooks: HooksResponse = from_json(
        module
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
            message_info(&Addr::unchecked("cosmwasm15fvnexrvsm9ryw3nn4mcrnqyhvhazkkr48xnwtlegjz2uaxmhg0sjzfhfz"), &[]),
            ExecuteMsg::Extension {
                msg: Empty::default(),
            },
        )
        .unwrap();
    assert_eq!(res, Response::default())
}
