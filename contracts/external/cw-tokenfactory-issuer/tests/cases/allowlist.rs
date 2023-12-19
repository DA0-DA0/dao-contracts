use cw_tokenfactory_issuer::{msg::StatusInfo, ContractError};
use osmosis_test_tube::Account;

use crate::test_env::{
    test_query_over_default_limit, test_query_within_default_limit, TestEnv, TokenfactoryIssuer,
};

#[cfg(feature = "osmosis_tokenfactory")]
#[test]
fn allowlist_by_owner_should_pass() {
    let env = TestEnv::default();
    let owner = &env.test_accs[0];
    let allowlistee = &env.test_accs[2];

    // Owner sets before send hook to enable allowlist feature
    env.cw_tokenfactory_issuer
        .set_before_send_hook(env.cw_tokenfactory_issuer.contract_addr.clone(), owner)
        .unwrap();

    env.cw_tokenfactory_issuer
        .allow(&allowlistee.address(), true, owner)
        .unwrap();

    // Should be allowlist after set true
    assert!(
        env.cw_tokenfactory_issuer
            .query_is_allowed(&allowlistee.address())
            .unwrap()
            .status
    );

    env.cw_tokenfactory_issuer
        .allow(&allowlistee.address(), false, owner)
        .unwrap();

    // Should be unallowlist after set false
    assert!(
        !env.cw_tokenfactory_issuer
            .query_is_allowed(&allowlistee.address())
            .unwrap()
            .status
    );
}

#[cfg(feature = "osmosis_tokenfactory")]
#[test]
fn allowlist_by_non_owern_should_fail() {
    let env = TestEnv::default();
    let owner = &env.test_accs[0];
    let non_owner = &env.test_accs[1];
    let allowlistee = &env.test_accs[2];

    // Owner sets before send hook to enable allowlist feature
    env.cw_tokenfactory_issuer
        .set_before_send_hook(env.cw_tokenfactory_issuer.contract_addr.clone(), owner)
        .unwrap();

    // Non-owner cannot add address to allowlist
    let err = env
        .cw_tokenfactory_issuer
        .allow(&allowlistee.address(), true, non_owner)
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
fn query_allowlist_within_default_limit() {
    test_query_within_default_limit::<StatusInfo, _, _>(
        |(_, addr)| StatusInfo {
            address: addr.to_string(),
            status: true,
        },
        |env| {
            move |expected_result| {
                let owner = &env.test_accs[0];

                // Owner sets before send hook to enable allowlist feature
                env.cw_tokenfactory_issuer
                    .set_before_send_hook(env.cw_tokenfactory_issuer.contract_addr.clone(), owner)
                    .unwrap();

                // Allowlist the address
                env.cw_tokenfactory_issuer
                    .allow(&expected_result.address, true, owner)
                    .unwrap();
            }
        },
        |env| {
            move |start_after, limit| {
                env.cw_tokenfactory_issuer
                    .query_allowlist(start_after, limit)
                    .unwrap()
                    .allowlist
            }
        },
    );
}

#[cfg(feature = "osmosis_tokenfactory")]
#[test]
fn query_allowlist_over_default_limit() {
    test_query_over_default_limit::<StatusInfo, _, _>(
        |(_, addr)| StatusInfo {
            address: addr.to_string(),
            status: true,
        },
        |env| {
            move |expected_result| {
                let owner = &env.test_accs[0];

                // Owner sets before send hook to enable allowlist feature
                env.cw_tokenfactory_issuer
                    .set_before_send_hook(env.cw_tokenfactory_issuer.contract_addr.clone(), owner)
                    .unwrap();

                // Allowlist the address
                env.cw_tokenfactory_issuer
                    .allow(&expected_result.address, true, owner)
                    .unwrap();
            }
        },
        |env| {
            move |start_after, limit| {
                env.cw_tokenfactory_issuer
                    .query_allowlist(start_after, limit)
                    .unwrap()
                    .allowlist
            }
        },
    );
}
