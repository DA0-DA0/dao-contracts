use cw_tokenfactory_issuer::ContractError;

use crate::test_env::{TestEnv, TokenfactoryIssuer};

#[cfg(feature = "osmosis_tokenfactory")]
#[test]
fn freeze_by_owener_should_pass() {
    let env = TestEnv::default();
    let owner = &env.test_accs[0];

    // Owner sets before send hook to enable advanced features
    env.cw_tokenfactory_issuer
        .set_before_send_hook(env.cw_tokenfactory_issuer.contract_addr.clone(), owner)
        .unwrap();

    env.cw_tokenfactory_issuer.freeze(true, owner).unwrap();

    // Should be frozen after set true
    assert!(
        env.cw_tokenfactory_issuer
            .query_is_frozen()
            .unwrap()
            .is_frozen
    );

    env.cw_tokenfactory_issuer.freeze(false, owner).unwrap();

    // Should be unfrozen after set false
    assert!(
        !env.cw_tokenfactory_issuer
            .query_is_frozen()
            .unwrap()
            .is_frozen
    );
}

#[cfg(feature = "osmosis_tokenfactory")]
#[test]
fn freeze_by_non_owner_should_fail() {
    let env = TestEnv::default();
    let owner = &env.test_accs[0];
    let non_owner = &env.test_accs[1];

    // Owner sets before send hook to enable advanced features
    env.cw_tokenfactory_issuer
        .set_before_send_hook(env.cw_tokenfactory_issuer.contract_addr.clone(), owner)
        .unwrap();

    // Non-owner cannot freeze
    let err = env
        .cw_tokenfactory_issuer
        .freeze(true, non_owner)
        .unwrap_err();

    assert_eq!(
        err,
        TokenfactoryIssuer::execute_error(ContractError::Ownership(
            cw_ownable::OwnershipError::NotOwner
        ))
    );
}
