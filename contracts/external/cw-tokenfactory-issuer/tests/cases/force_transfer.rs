use cosmwasm_std::Uint128;
use cw_tokenfactory_issuer::ContractError;
use osmosis_test_tube::Account;

use crate::test_env::{TestEnv, TokenfactoryIssuer};

#[cfg(feature = "osmosis_tokenfactory")]
#[test]
fn test_force_transfer() {
    let env = TestEnv::default();
    let owner = &env.test_accs[0];
    let non_owner = &env.test_accs[1];

    // Give owner permission to mint tokens
    let allowance = 100000000000;
    env.cw_tokenfactory_issuer
        .set_minter(&owner.address(), allowance, owner)
        .unwrap();

    // Mint tokens for owner and non_owner
    env.cw_tokenfactory_issuer
        .mint(&owner.address(), 10000000, owner)
        .unwrap();
    env.cw_tokenfactory_issuer
        .mint(&non_owner.address(), 10000000, owner)
        .unwrap();

    // Non-owner cannot force transfer tokens
    let err = env
        .cw_tokenfactory_issuer
        .force_transfer(
            non_owner,
            Uint128::new(10000),
            owner.address(),
            non_owner.address(),
        )
        .unwrap_err();

    assert_eq!(
        err,
        TokenfactoryIssuer::execute_error(ContractError::Ownership(
            cw_ownable::OwnershipError::NotOwner
        ))
    );

    // Owner can force transfer tokens
    env.cw_tokenfactory_issuer
        .force_transfer(
            owner,
            Uint128::new(10000),
            non_owner.address(),
            owner.address(),
        )
        .unwrap();
}
