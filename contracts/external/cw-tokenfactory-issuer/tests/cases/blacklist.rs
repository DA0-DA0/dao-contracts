use cw_tokenfactory_issuer::{msg::StatusInfo, ContractError};
use osmosis_test_tube::Account;

use crate::test_env::{
    test_query_over_default_limit, test_query_within_default_limit, TestEnv, TokenfactoryIssuer,
};

#[test]
fn set_blacklister_performed_by_contract_owner_should_pass() {
    let env = TestEnv::default();
    let owner = &env.test_accs[0];
    let non_owner = &env.test_accs[1];

    env.cw_tokenfactory_issuer
        .set_blacklister(&non_owner.address(), true, owner)
        .unwrap();

    let is_blacklister = env
        .cw_tokenfactory_issuer
        .query_is_blacklister(&env.test_accs[1].address())
        .unwrap()
        .status;

    assert!(is_blacklister);

    env.cw_tokenfactory_issuer
        .set_blacklister(&non_owner.address(), false, owner)
        .unwrap();

    let is_blacklister = env
        .cw_tokenfactory_issuer
        .query_is_blacklister(&env.test_accs[1].address())
        .unwrap()
        .status;

    assert!(!is_blacklister);
}

#[test]
fn set_blacklister_performed_by_non_contract_owner_should_fail() {
    let env = TestEnv::default();
    let non_owner = &env.test_accs[1];

    let err = env
        .cw_tokenfactory_issuer
        .set_blacklister(&non_owner.address(), true, non_owner)
        .unwrap_err();

    assert_eq!(
        err,
        TokenfactoryIssuer::execute_error(ContractError::Unauthorized {})
    );
}

#[test]
fn set_blacklister_to_false_should_remove_it_from_storage() {
    let env = TestEnv::default();
    let owner = &env.test_accs[0];

    let mut sorted_addrs = env
        .test_accs
        .iter()
        .map(|acc| acc.address())
        .collect::<Vec<_>>();
    sorted_addrs.sort();

    env.cw_tokenfactory_issuer
        .set_blacklister(&sorted_addrs[0], true, owner)
        .unwrap();
    env.cw_tokenfactory_issuer
        .set_blacklister(&sorted_addrs[1], true, owner)
        .unwrap();

    assert_eq!(
        env.cw_tokenfactory_issuer
            .query_blacklister_allowances(None, None)
            .unwrap()
            .blacklisters,
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
        .set_blacklister(&sorted_addrs[1], false, owner)
        .unwrap();

    assert_eq!(
        env.cw_tokenfactory_issuer
            .query_blacklister_allowances(None, None)
            .unwrap()
            .blacklisters,
        vec![StatusInfo {
            address: sorted_addrs[0].clone(),
            status: true
        },]
    );

    assert!(
        !env.cw_tokenfactory_issuer
            .query_is_blacklister(&sorted_addrs[1])
            .unwrap()
            .status
    );
}

#[test]
fn blacklist_by_blacklister_should_pass() {
    let env = TestEnv::default();
    let owner = &env.test_accs[0];
    let non_owner = &env.test_accs[1];
    let blacklistee = &env.test_accs[2];

    env.cw_tokenfactory_issuer
        .set_blacklister(&non_owner.address(), true, owner)
        .unwrap();
    env.cw_tokenfactory_issuer
        .blacklist(&blacklistee.address(), true, non_owner)
        .unwrap();

    // should be blacklisted after set true
    assert!(
        env.cw_tokenfactory_issuer
            .query_is_blacklisted(&blacklistee.address())
            .unwrap()
            .status
    );

    env.cw_tokenfactory_issuer
        .blacklist(&blacklistee.address(), false, non_owner)
        .unwrap();

    // should be unblacklisted after set false
    assert!(
        !env.cw_tokenfactory_issuer
            .query_is_blacklisted(&blacklistee.address())
            .unwrap()
            .status
    );
}

#[test]
fn blacklist_by_non_blacklister_should_fail() {
    let env = TestEnv::default();
    let owner = &env.test_accs[0];
    let blacklistee = &env.test_accs[2];
    let err = env
        .cw_tokenfactory_issuer
        .blacklist(&blacklistee.address(), true, owner)
        .unwrap_err();

    assert_eq!(
        err,
        TokenfactoryIssuer::execute_error(ContractError::Unauthorized {})
    );
}

#[test]
fn set_blacklist_to_false_should_remove_it_from_storage() {
    let env = TestEnv::default();
    let owner = &env.test_accs[0];

    let mut sorted_addrs = env
        .test_accs
        .iter()
        .map(|acc| acc.address())
        .collect::<Vec<_>>();
    sorted_addrs.sort();

    env.cw_tokenfactory_issuer
        .set_blacklister(&owner.address(), true, owner)
        .unwrap();

    env.cw_tokenfactory_issuer
        .blacklist(&sorted_addrs[0], true, owner)
        .unwrap();
    env.cw_tokenfactory_issuer
        .blacklist(&sorted_addrs[1], true, owner)
        .unwrap();

    assert_eq!(
        env.cw_tokenfactory_issuer
            .query_blacklistees(None, None)
            .unwrap()
            .blacklistees,
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
        .blacklist(&sorted_addrs[1], false, owner)
        .unwrap();

    assert_eq!(
        env.cw_tokenfactory_issuer
            .query_blacklistees(None, None)
            .unwrap()
            .blacklistees,
        vec![StatusInfo {
            address: sorted_addrs[0].clone(),
            status: true
        },]
    );

    assert!(
        !env.cw_tokenfactory_issuer
            .query_is_blacklisted(&sorted_addrs[1])
            .unwrap()
            .status
    );
}

// query blacklisters
#[test]
fn query_blacklister_within_default_limit() {
    test_query_within_default_limit::<StatusInfo, _, _>(
        |(_, addr)| StatusInfo {
            address: addr.to_string(),
            status: true,
        },
        |env| {
            move |allowance| {
                let owner = &env.test_accs[0];
                env.cw_tokenfactory_issuer
                    .set_blacklister(&allowance.address, true, owner)
                    .unwrap();
            }
        },
        |env| {
            move |start_after, limit| {
                env.cw_tokenfactory_issuer
                    .query_blacklister_allowances(start_after, limit)
                    .unwrap()
                    .blacklisters
            }
        },
    );
}

#[test]
fn query_blacklister_over_default_limit() {
    test_query_over_default_limit::<StatusInfo, _, _>(
        |(_, addr)| StatusInfo {
            address: addr.to_string(),
            status: true,
        },
        |env| {
            move |allowance| {
                let owner = &env.test_accs[0];
                env.cw_tokenfactory_issuer
                    .set_blacklister(&allowance.address, true, owner)
                    .unwrap();
            }
        },
        |env| {
            move |start_after, limit| {
                env.cw_tokenfactory_issuer
                    .query_blacklister_allowances(start_after, limit)
                    .unwrap()
                    .blacklisters
            }
        },
    );
}
// query blacklistees
#[test]
fn query_blacklistee_within_default_limit() {
    test_query_within_default_limit::<StatusInfo, _, _>(
        |(_, addr)| StatusInfo {
            address: addr.to_string(),
            status: true,
        },
        |env| {
            move |expected_result| {
                let owner = &env.test_accs[0];
                env.cw_tokenfactory_issuer
                    .set_blacklister(&owner.address(), true, owner)
                    .unwrap();

                env.cw_tokenfactory_issuer
                    .blacklist(&expected_result.address, true, owner)
                    .unwrap();
            }
        },
        |env| {
            move |start_after, limit| {
                env.cw_tokenfactory_issuer
                    .query_blacklistees(start_after, limit)
                    .unwrap()
                    .blacklistees
            }
        },
    );
}

#[test]
fn query_blacklistee_over_default_limit() {
    test_query_over_default_limit::<StatusInfo, _, _>(
        |(_, addr)| StatusInfo {
            address: addr.to_string(),
            status: true,
        },
        |env| {
            move |expected_result| {
                let owner = &env.test_accs[0];
                env.cw_tokenfactory_issuer
                    .set_blacklister(&owner.address(), true, owner)
                    .unwrap();

                env.cw_tokenfactory_issuer
                    .blacklist(&expected_result.address, true, owner)
                    .unwrap();
            }
        },
        |env| {
            move |start_after, limit| {
                env.cw_tokenfactory_issuer
                    .query_blacklistees(start_after, limit)
                    .unwrap()
                    .blacklistees
            }
        },
    );
}
