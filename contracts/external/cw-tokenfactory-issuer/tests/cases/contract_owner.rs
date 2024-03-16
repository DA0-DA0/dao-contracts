use cosmwasm_std::{Addr, Uint128};
use cw_tokenfactory_issuer::{msg::ExecuteMsg, ContractError};
use osmosis_test_tube::Account;

use crate::test_env::{TestEnv, TokenfactoryIssuer};

#[test]
fn change_owner_by_owner_should_work() {
    let env = TestEnv::default();
    let prev_owner = &env.test_accs[0];
    let new_owner = &env.test_accs[1];

    assert_eq!(
        Some(Addr::unchecked(prev_owner.address())),
        env.cw_tokenfactory_issuer.query_owner().unwrap().owner,
    );

    env.cw_tokenfactory_issuer
        .update_contract_owner(new_owner, prev_owner)
        .unwrap();

    assert_eq!(
        env.cw_tokenfactory_issuer.query_owner().unwrap().owner,
        Some(Addr::unchecked(new_owner.address())),
    );

    // Previous owner should not be able to execute owner action
    assert_eq!(
        env.cw_tokenfactory_issuer
            .update_contract_owner(prev_owner, prev_owner)
            .unwrap_err(),
        TokenfactoryIssuer::execute_error(ContractError::Ownership(
            cw_ownable::OwnershipError::NotOwner
        ))
    );
}

#[test]
fn change_owner_by_non_owner_should_fail() {
    let env = TestEnv::default();
    let new_owner = &env.test_accs[1];

    let err = env
        .cw_tokenfactory_issuer
        .update_contract_owner(new_owner, new_owner)
        .unwrap_err();

    assert_eq!(
        err,
        TokenfactoryIssuer::execute_error(ContractError::Ownership(
            cw_ownable::OwnershipError::NotOwner
        ))
    );
}

#[cfg(feature = "osmosis_tokenfactory")]
#[test]
fn renounce_ownership() {
    let env = TestEnv::default();
    let owner = &env.test_accs[0];
    let non_owner = &env.test_accs[1];
    let hook = &env.test_accs[1];

    assert_eq!(
        Some(Addr::unchecked(owner.address())),
        env.cw_tokenfactory_issuer.query_owner().unwrap().owner,
    );

    // Renounce ownership
    env.cw_tokenfactory_issuer
        .execute(
            &ExecuteMsg::UpdateOwnership(cw_ownable::Action::RenounceOwnership),
            &[],
            owner,
        )
        .unwrap();

    assert_eq!(
        env.cw_tokenfactory_issuer.query_owner().unwrap().owner,
        None,
    );

    // Cannot perform actions that require ownership
    assert_eq!(
        env.cw_tokenfactory_issuer
            .set_minter(&non_owner.address(), 10000, owner)
            .unwrap_err(),
        TokenfactoryIssuer::execute_error(ContractError::Ownership(
            cw_ownable::OwnershipError::NoOwner
        ))
    );
    assert_eq!(
        env.cw_tokenfactory_issuer
            .set_burner(&non_owner.address(), 10000, owner)
            .unwrap_err(),
        TokenfactoryIssuer::execute_error(ContractError::Ownership(
            cw_ownable::OwnershipError::NoOwner
        ))
    );
    assert_eq!(
        env.cw_tokenfactory_issuer
            .force_transfer(
                non_owner,
                Uint128::new(10000),
                owner.address(),
                non_owner.address(),
            )
            .unwrap_err(),
        TokenfactoryIssuer::execute_error(ContractError::Ownership(
            cw_ownable::OwnershipError::NoOwner
        ))
    );
    assert_eq!(
        env.cw_tokenfactory_issuer
            .set_before_send_hook(hook.address(), owner)
            .unwrap_err(),
        TokenfactoryIssuer::execute_error(ContractError::Ownership(
            cw_ownable::OwnershipError::NoOwner
        ))
    );
}
