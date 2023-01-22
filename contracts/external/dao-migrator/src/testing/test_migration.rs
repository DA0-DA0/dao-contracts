use std::borrow::BorrowMut;

use cw_multi_test::App;

use crate::ContractError;

use super::setup::{execute_migration, init_dao_v1};

#[test]
fn test_execute_migration() {
    let app = App::default();

    let (mut app, core_addr, proposal_addr, v1_code_ids) = init_dao_v1(app, None);

    execute_migration(
        app.borrow_mut(),
        core_addr.as_ref(),
        proposal_addr.as_ref(),
        v1_code_ids,
    )
    .unwrap();
}

#[test]
fn test_wrong_code_id() {
    let app = App::default();

    let (mut app, core_addr, proposal_addr, mut v1_code_ids) = init_dao_v1(app, None);

    v1_code_ids.proposal_single = 555;
    let err = execute_migration(
        app.borrow_mut(),
        core_addr.as_ref(),
        proposal_addr.as_ref(),
        v1_code_ids.clone(),
    )
    .unwrap_err()
    .downcast::<ContractError>()
    .unwrap();
    assert_eq!(err, ContractError::CantMigrateModule { code_id: 2 });

    v1_code_ids.cw20_stake = 555;
    let err = execute_migration(
        app.borrow_mut(),
        core_addr.as_ref(),
        proposal_addr.as_ref(),
        v1_code_ids.clone(),
    )
    .unwrap_err()
    .downcast::<ContractError>()
    .unwrap();
    assert_eq!(err, ContractError::CantMigrateModule { code_id: 4 });

    v1_code_ids.cw20_staked_balances_voting = 555;
    let err = execute_migration(
        app.borrow_mut(),
        core_addr.as_ref(),
        proposal_addr.as_ref(),
        v1_code_ids.clone(),
    )
    .unwrap_err()
    .downcast::<ContractError>()
    .unwrap();
    assert_eq!(err, ContractError::VotingModuleNotFound);

    v1_code_ids.cw4_voting = 555;
    let err = execute_migration(
        app.borrow_mut(),
        core_addr.as_ref(),
        proposal_addr.as_ref(),
        v1_code_ids,
    )
    .unwrap_err()
    .downcast::<ContractError>()
    .unwrap();
    assert_eq!(err, ContractError::VotingModuleNotFound);
}
