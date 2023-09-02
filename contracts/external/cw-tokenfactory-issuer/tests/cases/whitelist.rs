use cw_tokenfactory_issuer::{msg::StatusInfo, ContractError};
use osmosis_test_tube::Account;

use crate::test_env::{
    test_query_over_default_limit, test_query_within_default_limit, TestEnv, TokenfactoryIssuer,
};

#[test]
fn set_whitelister_performed_by_contract_owner_should_pass() {
    let env = TestEnv::default();
    let owner = &env.test_accs[0];
    let non_owner = &env.test_accs[1];

    env.cw_tokenfactory_issuer
        .set_whitelister(&non_owner.address(), true, owner)
        .unwrap();

    let is_whitelister = env
        .cw_tokenfactory_issuer
        .query_is_whitelister(&env.test_accs[1].address())
        .unwrap()
        .status;

    assert!(is_whitelister);

    env.cw_tokenfactory_issuer
        .set_whitelister(&non_owner.address(), false, owner)
        .unwrap();

    let is_whitelister = env
        .cw_tokenfactory_issuer
        .query_is_whitelister(&env.test_accs[1].address())
        .unwrap()
        .status;

    assert!(!is_whitelister);
}

#[test]
fn set_whitelister_performed_by_non_contract_owner_should_fail() {
    let env = TestEnv::default();
    let non_owner = &env.test_accs[1];

    let err = env
        .cw_tokenfactory_issuer
        .set_whitelister(&non_owner.address(), true, non_owner)
        .unwrap_err();

    assert_eq!(
        err,
        TokenfactoryIssuer::execute_error(ContractError::Unauthorized {})
    );
}

#[test]
fn set_whitelister_to_false_should_remove_it_from_storage() {
    let env = TestEnv::default();
    let owner = &env.test_accs[0];

    let mut sorted_addrs = env
        .test_accs
        .iter()
        .map(|acc| acc.address())
        .collect::<Vec<_>>();
    sorted_addrs.sort();

    env.cw_tokenfactory_issuer
        .set_whitelister(&sorted_addrs[0], true, owner)
        .unwrap();
    env.cw_tokenfactory_issuer
        .set_whitelister(&sorted_addrs[1], true, owner)
        .unwrap();

    assert_eq!(
        env.cw_tokenfactory_issuer
            .query_whitelisters(None, None)
            .unwrap()
            .whitelisters,
        vec![
            StatusInfo {
                address: sorted_addrs[0].clone(),
                status: true
            },
            StatusInfo {
                address: sorted_addrs[1].clone(),
                status: true
            }
        ]
    );

    env.cw_tokenfactory_issuer
        .set_whitelister(&sorted_addrs[1], false, owner)
        .unwrap();

    assert_eq!(
        env.cw_tokenfactory_issuer
            .query_whitelisters(None, None)
            .unwrap()
            .whitelisters,
        vec![StatusInfo {
            address: sorted_addrs[0].clone(),
            status: true
        },]
    );

    assert!(
        !env.cw_tokenfactory_issuer
            .query_is_whitelister(&sorted_addrs[1])
            .unwrap()
            .status
    );
}

#[test]
fn whitelist_by_whitelister_should_pass() {
    let env = TestEnv::default();
    let owner = &env.test_accs[0];
    let non_owner = &env.test_accs[1];
    let whitelistee = &env.test_accs[2];

    env.cw_tokenfactory_issuer
        .set_whitelister(&non_owner.address(), true, owner)
        .unwrap();
    env.cw_tokenfactory_issuer
        .whitelist(&whitelistee.address(), true, non_owner)
        .unwrap();

    // should be whitelisted after set true
    assert!(
        env.cw_tokenfactory_issuer
            .query_is_whitelisted(&whitelistee.address())
            .unwrap()
            .status
    );

    env.cw_tokenfactory_issuer
        .whitelist(&whitelistee.address(), false, non_owner)
        .unwrap();

    // should be unwhitelisted after set false
    assert!(
        !env.cw_tokenfactory_issuer
            .query_is_whitelisted(&whitelistee.address())
            .unwrap()
            .status
    );
}

#[test]
fn whitelist_by_non_whitelister_should_fail() {
    let env = TestEnv::default();
    let owner = &env.test_accs[0];
    let whitelistee = &env.test_accs[2];
    let err = env
        .cw_tokenfactory_issuer
        .whitelist(&whitelistee.address(), true, owner)
        .unwrap_err();

    assert_eq!(
        err,
        TokenfactoryIssuer::execute_error(ContractError::Unauthorized {})
    );
}

#[test]
fn set_whitelist_to_false_should_remove_it_from_storage() {
    let env = TestEnv::default();
    let owner = &env.test_accs[0];

    let mut sorted_addrs = env
        .test_accs
        .iter()
        .map(|acc| acc.address())
        .collect::<Vec<_>>();
    sorted_addrs.sort();

    env.cw_tokenfactory_issuer
        .set_whitelister(&owner.address(), true, owner)
        .unwrap();

    env.cw_tokenfactory_issuer
        .whitelist(&sorted_addrs[0], true, owner)
        .unwrap();
    env.cw_tokenfactory_issuer
        .whitelist(&sorted_addrs[1], true, owner)
        .unwrap();

    assert_eq!(
        env.cw_tokenfactory_issuer
            .query_whitelistees(None, None)
            .unwrap()
            .whitelistees,
        vec![
            StatusInfo {
                address: sorted_addrs[0].clone(),
                status: true
            },
            StatusInfo {
                address: sorted_addrs[1].clone(),
                status: true
            }
        ]
    );

    env.cw_tokenfactory_issuer
        .whitelist(&sorted_addrs[1], false, owner)
        .unwrap();

    assert_eq!(
        env.cw_tokenfactory_issuer
            .query_whitelistees(None, None)
            .unwrap()
            .whitelistees,
        vec![StatusInfo {
            address: sorted_addrs[0].clone(),
            status: true
        },]
    );

    assert!(
        !env.cw_tokenfactory_issuer
            .query_is_whitelisted(&sorted_addrs[1])
            .unwrap()
            .status
    );
}

// query whitelisters
#[test]
fn query_whitelister_within_default_limit() {
    test_query_within_default_limit::<StatusInfo, _, _>(
        |(_, addr)| StatusInfo {
            address: addr.to_string(),
            status: true,
        },
        |env| {
            move |allowance| {
                let owner = &env.test_accs[0];
                env.cw_tokenfactory_issuer
                    .set_whitelister(&allowance.address, true, owner)
                    .unwrap();
            }
        },
        |env| {
            move |start_after, limit| {
                env.cw_tokenfactory_issuer
                    .query_whitelisters(start_after, limit)
                    .unwrap()
                    .whitelisters
            }
        },
    );
}

#[test]
fn query_whitelister_over_default_limit() {
    test_query_over_default_limit::<StatusInfo, _, _>(
        |(_, addr)| StatusInfo {
            address: addr.to_string(),
            status: true,
        },
        |env| {
            move |allowance| {
                let owner = &env.test_accs[0];
                env.cw_tokenfactory_issuer
                    .set_whitelister(&allowance.address, true, owner)
                    .unwrap();
            }
        },
        |env| {
            move |start_after, limit| {
                env.cw_tokenfactory_issuer
                    .query_whitelisters(start_after, limit)
                    .unwrap()
                    .whitelisters
            }
        },
    );
}
// query whitelistees
#[test]
fn query_whitelistee_within_default_limit() {
    test_query_within_default_limit::<StatusInfo, _, _>(
        |(_, addr)| StatusInfo {
            address: addr.to_string(),
            status: true,
        },
        |env| {
            move |expected_result| {
                let owner = &env.test_accs[0];
                env.cw_tokenfactory_issuer
                    .set_whitelister(&owner.address(), true, owner)
                    .unwrap();

                env.cw_tokenfactory_issuer
                    .whitelist(&expected_result.address, true, owner)
                    .unwrap();
            }
        },
        |env| {
            move |start_after, limit| {
                env.cw_tokenfactory_issuer
                    .query_whitelistees(start_after, limit)
                    .unwrap()
                    .whitelistees
            }
        },
    );
}

#[test]
fn query_whitelistee_over_default_limit() {
    test_query_over_default_limit::<StatusInfo, _, _>(
        |(_, addr)| StatusInfo {
            address: addr.to_string(),
            status: true,
        },
        |env| {
            move |expected_result| {
                let owner = &env.test_accs[0];
                env.cw_tokenfactory_issuer
                    .set_whitelister(&owner.address(), true, owner)
                    .unwrap();

                env.cw_tokenfactory_issuer
                    .whitelist(&expected_result.address, true, owner)
                    .unwrap();
            }
        },
        |env| {
            move |start_after, limit| {
                env.cw_tokenfactory_issuer
                    .query_whitelistees(start_after, limit)
                    .unwrap()
                    .whitelistees
            }
        },
    );
}
