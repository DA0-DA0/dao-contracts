use cw_tokenfactory_issuer::{msg::StatusInfo, ContractError};
use osmosis_test_tube::Account;

use crate::test_env::{
    test_query_over_default_limit, test_query_within_default_limit, TestEnv, TokenfactoryIssuer,
};

#[test]
fn freeze_by_owener_should_pass() {
    let env = TestEnv::default();
    let owner = &env.test_accs[0];

    env.cw_tokenfactory_issuer.freeze(true, owner).unwrap();

    // should be frozen after set true
    assert!(
        env.cw_tokenfactory_issuer
            .query_is_frozen()
            .unwrap()
            .is_frozen
    );

    env.cw_tokenfactory_issuer.freeze(false, owner).unwrap();

    // should be unfrozen after set false
    assert!(
        !env.cw_tokenfactory_issuer
            .query_is_frozen()
            .unwrap()
            .is_frozen
    );
}

#[test]
fn freeze_by_non_freezer_should_fail() {
    let env = TestEnv::default();
    let non_owner = &env.test_accs[1];
    let err = env
        .cw_tokenfactory_issuer
        .freeze(true, non_owner)
        .unwrap_err();

    assert_eq!(
        err,
        TokenfactoryIssuer::execute_error(ContractError::Unauthorized {})
    );
}
