use std::borrow::BorrowMut;

use dao_core::{query::SubDao, state::ProposalModuleStatus};

use crate::{
    testing::{
        helpers::ExecuteParams,
        helpers::VotingType,
        setup::{execute_migration, execute_migration_from_core, setup_dao_v1},
        state_helpers::{
            query_state_v1_cw20, query_state_v1_cw4, query_state_v2_cw20, query_state_v2_cw4,
        },
    },
    ContractError,
};

pub fn basic_test(voting_type: VotingType, from_core: bool) {
    let (mut app, module_addrs, v1_code_ids) = setup_dao_v1(voting_type.clone());

    let mut test_state_v1 = match voting_type {
        VotingType::Cw4 => query_state_v1_cw4(
            &mut app,
            module_addrs.proposal.clone(),
            module_addrs.voting.clone(),
        ),
        VotingType::Cw20 => query_state_v1_cw20(
            &mut app,
            module_addrs.proposal.clone(),
            module_addrs.voting.clone(),
        ),
    };
    //NOTE: We add 1 to count because we create a new proposal in execute_migration
    test_state_v1.proposal_count += 1;

    match from_core {
        true => {
            execute_migration_from_core(app.borrow_mut(), &module_addrs, v1_code_ids, None).unwrap()
        }
        false => execute_migration(app.borrow_mut(), &module_addrs, v1_code_ids, None).unwrap(),
    };

    let test_state_v2 = match voting_type {
        VotingType::Cw4 => {
            query_state_v2_cw4(&mut app, module_addrs.proposal.clone(), module_addrs.voting)
        }
        VotingType::Cw20 => {
            query_state_v2_cw20(&mut app, module_addrs.proposal.clone(), module_addrs.voting)
        }
    };

    assert_eq!(test_state_v1, test_state_v2);

    let modules: Vec<dao_core::state::ProposalModule> = app
        .wrap()
        .query_wasm_smart(
            module_addrs.core,
            &dao_core::msg::QueryMsg::ProposalModules {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();
    assert_eq!(modules.len(), 2);
    assert_eq!(modules[0].address, module_addrs.proposal);
    assert_eq!(modules[1].status, ProposalModuleStatus::Disabled);
}

#[test]
fn test_execute_migration() {
    // Test basic migrator (not called from core)
    basic_test(VotingType::Cw20, false);
    basic_test(VotingType::Cw4, false);

    // Test basic migrator (called from core)
    basic_test(VotingType::Cw20, true);
    basic_test(VotingType::Cw4, true);
}

#[test]
fn test_wrong_code_id() {
    let (mut app, module_addrs, mut v1_code_ids) = setup_dao_v1(VotingType::Cw20);
    let old_v1_code_ids = v1_code_ids.clone();
    v1_code_ids.proposal_single = 555;
    let err = execute_migration(app.borrow_mut(), &module_addrs, v1_code_ids, None)
        .unwrap_err()
        .downcast::<ContractError>()
        .unwrap();
    assert_eq!(
        err,
        ContractError::CantMigrateModule {
            code_id: old_v1_code_ids.proposal_single
        }
    );

    let (mut app, module_addrs, mut v1_code_ids) = setup_dao_v1(VotingType::Cw20);
    let old_v1_code_ids = v1_code_ids.clone();
    v1_code_ids.cw20_stake = 555;
    let err = execute_migration(app.borrow_mut(), &module_addrs, v1_code_ids, None)
        .unwrap_err()
        .downcast::<ContractError>()
        .unwrap();
    assert_eq!(
        err,
        ContractError::CantMigrateModule {
            code_id: old_v1_code_ids.cw20_stake
        }
    );

    let (mut app, module_addrs, mut v1_code_ids) = setup_dao_v1(VotingType::Cw20);
    v1_code_ids.cw20_staked_balances_voting = 555;
    let err = execute_migration(app.borrow_mut(), &module_addrs, v1_code_ids, None)
        .unwrap_err()
        .downcast::<ContractError>()
        .unwrap();
    assert_eq!(err, ContractError::VotingModuleNotFound);

    let (mut app, module_addrs, mut v1_code_ids) = setup_dao_v1(VotingType::Cw4);
    v1_code_ids.cw4_voting = 555;
    let err = execute_migration(app.borrow_mut(), &module_addrs, v1_code_ids, None)
        .unwrap_err()
        .downcast::<ContractError>()
        .unwrap();
    assert_eq!(err, ContractError::VotingModuleNotFound);
}

#[test]
fn test_dont_migrate_cw20() {
    let (mut app, module_addrs, v1_code_ids) = setup_dao_v1(VotingType::Cw20);

    let err = execute_migration(
        app.borrow_mut(),
        &module_addrs,
        v1_code_ids.clone(),
        Some(ExecuteParams {
            sub_daos: Some(vec![]),
            migrate_cw20: None,
        }),
    )
    .unwrap_err()
    .downcast::<ContractError>()
    .unwrap();
    assert_eq!(err, ContractError::DontMigrateCw20);

    let err = execute_migration(
        app.borrow_mut(),
        &module_addrs,
        v1_code_ids,
        Some(ExecuteParams {
            sub_daos: Some(vec![]),
            migrate_cw20: Some(false),
        }),
    )
    .unwrap_err()
    .downcast::<ContractError>()
    .unwrap();
    assert_eq!(err, ContractError::DontMigrateCw20);
}

#[test]
fn test_sub_daos() {
    let (mut app, module_addrs, v1_code_ids) = setup_dao_v1(VotingType::Cw20);
    let sub_dao = SubDao {
        addr: "sub_dao_1".to_string(),
        charter: None,
    };

    execute_migration(
        app.borrow_mut(),
        &module_addrs,
        v1_code_ids,
        Some(ExecuteParams {
            sub_daos: Some(vec![sub_dao.clone()]),
            migrate_cw20: Some(true),
        }),
    )
    .unwrap();

    let sub_daos: Vec<dao_core::query::SubDao> = app
        .wrap()
        .query_wasm_smart(
            module_addrs.core,
            &dao_core::msg::QueryMsg::ListSubDaos {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(sub_daos, vec![sub_dao]);
}
