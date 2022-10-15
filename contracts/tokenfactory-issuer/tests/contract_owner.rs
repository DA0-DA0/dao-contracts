mod helpers;
use helpers::{TestEnv, TokenfactoryIssuer};
use osmosis_testing::Account;
use tokenfactory_issuer::ContractError;

#[test]
fn change_owner_by_owner_should_work() {
    let env = TestEnv::default();
    let prev_owner = &env.test_accs[0];
    let new_owner = &env.test_accs[1];

    assert_eq!(
        prev_owner.address(),
        env.tokenfactory_issuer.query_owner().unwrap().address
    );

    env.tokenfactory_issuer
        .change_contract_owner(&new_owner.address(), prev_owner)
        .unwrap();

    assert_eq!(
        new_owner.address(),
        env.tokenfactory_issuer.query_owner().unwrap().address
    );

    // previous owner should not be able to execute owner action
    assert_eq!(
        env.tokenfactory_issuer
            .change_contract_owner(&prev_owner.address(), prev_owner)
            .unwrap_err(),
        TokenfactoryIssuer::execute_error(ContractError::Unauthorized {})
    );
}

#[test]
fn change_owner_by_non_owner_should_fail() {
    let env = TestEnv::default();
    let new_owner = &env.test_accs[1];

    let err = env
        .tokenfactory_issuer
        .change_contract_owner(&new_owner.address(), new_owner)
        .unwrap_err();

    assert_eq!(
        err,
        TokenfactoryIssuer::execute_error(ContractError::Unauthorized {})
    );
}
