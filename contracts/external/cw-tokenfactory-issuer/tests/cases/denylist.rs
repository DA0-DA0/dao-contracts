use cw_tokenfactory_issuer::{msg::StatusInfo, ContractError};
use osmosis_test_tube::Account;

use crate::test_env::{
    test_query_over_default_limit, test_query_within_default_limit, TestEnv, TokenfactoryIssuer,
};

#[cfg(feature = "osmosis_tokenfactory")]
#[test]
fn denylist_by_owner_should_pass() {
    let env = TestEnv::default();
    let owner = &env.test_accs[0];
    let denylistee = &env.test_accs[2];

    // Owner sets before send hook to enable advanced features
    env.cw_tokenfactory_issuer
        .set_before_send_hook(env.cw_tokenfactory_issuer.contract_addr.clone(), owner)
        .unwrap();

    env.cw_tokenfactory_issuer
        .deny(&denylistee.address(), true, owner)
        .unwrap();

    // Should be denylist after set true
    assert!(
        env.cw_tokenfactory_issuer
            .query_is_denied(&denylistee.address())
            .unwrap()
            .status
    );

    env.cw_tokenfactory_issuer
        .deny(&denylistee.address(), false, owner)
        .unwrap();

    // Should be undenylist after set false
    assert!(
        !env.cw_tokenfactory_issuer
            .query_is_denied(&denylistee.address())
            .unwrap()
            .status
    );
}

#[cfg(feature = "osmosis_tokenfactory")]
#[test]
fn denylist_by_non_owner_should_fail() {
    let env = TestEnv::default();
    let owner = &env.test_accs[0];
    let non_owner = &env.test_accs[1];
    let denylistee = &env.test_accs[2];

    // Owner sets before send hook to enable advanced features
    env.cw_tokenfactory_issuer
        .set_before_send_hook(env.cw_tokenfactory_issuer.contract_addr.clone(), owner)
        .unwrap();

    // Non-owner cannot add address to denylist
    let err = env
        .cw_tokenfactory_issuer
        .deny(&denylistee.address(), true, non_owner)
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
fn set_denylist_to_issuer_itself_fails() {
    let env = TestEnv::default();
    let owner = &env.test_accs[0];

    // Owner sets before send hook to enable advanced features
    env.cw_tokenfactory_issuer
        .set_before_send_hook(env.cw_tokenfactory_issuer.contract_addr.clone(), owner)
        .unwrap();

    // Owner cannot deny issuer itself
    let err = env
        .cw_tokenfactory_issuer
        .deny(&env.cw_tokenfactory_issuer.contract_addr, true, owner)
        .unwrap_err();

    assert_eq!(
        err,
        TokenfactoryIssuer::execute_error(ContractError::CannotDenylistSelf {})
    );
}

#[cfg(feature = "osmosis_tokenfactory")]
#[test]
fn query_denylist_within_default_limit() {
    test_query_within_default_limit::<StatusInfo, _, _>(
        |(_, addr)| StatusInfo {
            address: addr.to_string(),
            status: true,
        },
        |env| {
            move |expected_result| {
                let owner = &env.test_accs[0];

                // Owner sets before send hook to enable advanced features
                env.cw_tokenfactory_issuer
                    .set_before_send_hook(env.cw_tokenfactory_issuer.contract_addr.clone(), owner)
                    .unwrap();

                // Deny address
                env.cw_tokenfactory_issuer
                    .deny(&expected_result.address, true, owner)
                    .unwrap();
            }
        },
        |env| {
            move |start_after, limit| {
                env.cw_tokenfactory_issuer
                    .query_denylist(start_after, limit)
                    .unwrap()
                    .denylist
            }
        },
    );
}

#[cfg(feature = "osmosis_tokenfactory")]
#[test]
fn query_denylist_over_default_limit() {
    test_query_over_default_limit::<StatusInfo, _, _>(
        |(_, addr)| StatusInfo {
            address: addr.to_string(),
            status: true,
        },
        |env| {
            move |expected_result| {
                let owner = &env.test_accs[0];

                // Owner sets before send hook to enable advanced features
                env.cw_tokenfactory_issuer
                    .set_before_send_hook(env.cw_tokenfactory_issuer.contract_addr.clone(), owner)
                    .unwrap();

                // Deny address
                env.cw_tokenfactory_issuer
                    .deny(&expected_result.address, true, owner)
                    .unwrap();
            }
        },
        |env| {
            move |start_after, limit| {
                env.cw_tokenfactory_issuer
                    .query_denylist(start_after, limit)
                    .unwrap()
                    .denylist
            }
        },
    );
}
