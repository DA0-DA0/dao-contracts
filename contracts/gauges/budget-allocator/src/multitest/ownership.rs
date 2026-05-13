use cw_ownable::{Action, Ownership, OwnershipError};

use crate::{
    msg::{ExecuteMsg, QueryMsg},
    multitest::suite::{addr, ujuno, Suite},
    ContractError,
};

#[test]
fn ownership_starts_with_instantiator() {
    let suite = Suite::new(&["alice"], ujuno(1_000));
    let owner = suite.owner.clone();
    let ownership: Ownership<cosmwasm_std::Addr> = suite.query(&QueryMsg::Ownership {}).unwrap();
    assert_eq!(ownership.owner, Some(owner));
    assert!(ownership.pending_owner.is_none());
}

#[test]
fn transfer_ownership_two_step_flow() {
    let mut suite = Suite::new(&["alice"], ujuno(1_000));
    let old_owner = suite.owner.clone();
    let new_owner = addr("new_owner");

    suite
        .execute_owner(&ExecuteMsg::UpdateOwnership(Action::TransferOwnership {
            new_owner: new_owner.to_string(),
            expiry: None,
        }))
        .unwrap();

    // Before accept: new owner has no authority.
    let err = suite
        .execute_as(
            &new_owner,
            &ExecuteMsg::AddOption {
                option: "bob".into(),
            },
        )
        .unwrap_err();
    assert_eq!(
        ContractError::Ownership(OwnershipError::NotOwner),
        err.downcast().unwrap()
    );

    // Accept.
    suite
        .execute_as(
            &new_owner,
            &ExecuteMsg::UpdateOwnership(Action::AcceptOwnership),
        )
        .unwrap();

    // After accept: new owner works, old owner does not.
    suite
        .execute_as(
            &new_owner,
            &ExecuteMsg::AddOption {
                option: "bob".into(),
            },
        )
        .unwrap();
    let err = suite
        .execute_as(
            &old_owner,
            &ExecuteMsg::AddOption {
                option: "carol".into(),
            },
        )
        .unwrap_err();
    assert_eq!(
        ContractError::Ownership(OwnershipError::NotOwner),
        err.downcast().unwrap()
    );
}

#[test]
fn renounce_ownership_locks_owner_methods() {
    let mut suite = Suite::new(&["alice"], ujuno(1_000));
    let owner = suite.owner.clone();

    suite
        .execute_owner(&ExecuteMsg::UpdateOwnership(Action::RenounceOwnership))
        .unwrap();

    let err = suite
        .execute_as(
            &owner,
            &ExecuteMsg::AddOption {
                option: "bob".into(),
            },
        )
        .unwrap_err();
    assert_eq!(
        ContractError::Ownership(OwnershipError::NoOwner),
        err.downcast().unwrap()
    );

    let ownership: Ownership<cosmwasm_std::Addr> = suite.query(&QueryMsg::Ownership {}).unwrap();
    assert!(ownership.owner.is_none());
}
