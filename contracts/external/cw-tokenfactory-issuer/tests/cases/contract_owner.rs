use cw_tokenfactory_issuer::ContractError;
use osmosis_test_tube::Account;

use crate::test_env::{TestEnv, TokenfactoryIssuer};

#[test]
fn change_owner_by_owner_should_work() {
    let env = TestEnv::default();
    let prev_owner = &env.test_accs[0];
    let new_owner = &env.test_accs[1];

    assert_eq!(
        prev_owner.address(),
        env.cw_tokenfactory_issuer.query_owner().unwrap().address
    );

    env.cw_tokenfactory_issuer
        .update_contract_owner(&new_owner.address(), prev_owner)
        .unwrap();

    assert_eq!(
        new_owner.address(),
        env.cw_tokenfactory_issuer.query_owner().unwrap().address
    );

    // Previous owner should not be able to execute owner action
    assert_eq!(
        env.cw_tokenfactory_issuer
            .update_contract_owner(&prev_owner.address(), prev_owner)
            .unwrap_err(),
        TokenfactoryIssuer::execute_error(ContractError::Unauthorized {})
    );
}

#[test]
fn change_owner_by_non_owner_should_fail() {
    let env = TestEnv::default();
    let new_owner = &env.test_accs[1];

    let err = env
        .cw_tokenfactory_issuer
        .update_contract_owner(&new_owner.address(), new_owner)
        .unwrap_err();

    assert_eq!(
        err,
        TokenfactoryIssuer::execute_error(ContractError::Unauthorized {})
    );
}
