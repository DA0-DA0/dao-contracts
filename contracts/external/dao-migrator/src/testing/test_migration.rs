use std::borrow::BorrowMut;

use dao_core::query::SubDao;

use crate::{testing::helpers::ExecuteParams, ContractError};

use super::{
    helpers::VotingType,
    setup::{execute_migration, setup_dao_v1},
};

#[test]
fn test_execute_migration() {
    let (mut app, module_addrs, v1_code_ids) = setup_dao_v1(VotingType::Cw20);

    execute_migration(app.borrow_mut(), &module_addrs, v1_code_ids, None).unwrap();
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

    execute_migration(
        app.borrow_mut(),
        &module_addrs,
        v1_code_ids,
        Some(ExecuteParams {
            sub_daos: Some(vec![SubDao {
                addr: "sub_dao_1".to_string(),
                charter: None,
            }]),
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

    assert_eq!(sub_daos.len(), 1);
    assert_eq!(
        sub_daos,
        vec![SubDao {
            addr: "sub_dao_1".to_string(),
            charter: None,
        }]
    );
}
