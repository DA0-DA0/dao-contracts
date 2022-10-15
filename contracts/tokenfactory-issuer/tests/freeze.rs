mod helpers;

use helpers::{TestEnv, TokenfactoryIssuer};
use osmosis_testing::Account;
use tokenfactory_issuer::{msg::StatusInfo, ContractError};

#[test]
fn set_freezeer_performed_by_contract_owner_should_pass() {
    let env = TestEnv::default();
    let owner = &env.test_accs[0];
    let non_owner = &env.test_accs[1];

    env.tokenfactory_issuer
        .set_freezer(&non_owner.address(), true, owner)
        .unwrap();

    let is_freezer = env
        .tokenfactory_issuer
        .query_is_freezer(&env.test_accs[1].address())
        .unwrap()
        .status;

    assert!(is_freezer);

    env.tokenfactory_issuer
        .set_freezer(&non_owner.address(), false, owner)
        .unwrap();

    let is_freezer = env
        .tokenfactory_issuer
        .query_is_freezer(&env.test_accs[1].address())
        .unwrap()
        .status;

    assert!(!is_freezer);
}

#[test]
fn freeze_by_freezer_should_pass() {
    let env = TestEnv::default();
    let owner = &env.test_accs[0];
    let non_owner = &env.test_accs[1];

    env.tokenfactory_issuer
        .set_freezer(&non_owner.address(), true, owner)
        .unwrap();
    env.tokenfactory_issuer.freeze(true, non_owner).unwrap();

    // should be frozen after set true
    assert!(env.tokenfactory_issuer.query_is_frozen().unwrap().is_frozen);

    env.tokenfactory_issuer.freeze(false, non_owner).unwrap();

    // should be unfrozen after set false
    assert!(!env.tokenfactory_issuer.query_is_frozen().unwrap().is_frozen);
}

#[test]
fn freeze_by_non_freezer_should_fail() {
    let env = TestEnv::default();
    let owner = &env.test_accs[0];
    let err = env.tokenfactory_issuer.freeze(true, owner).unwrap_err();

    assert_eq!(
        err,
        TokenfactoryIssuer::execute_error(ContractError::Unauthorized {})
    );
}

#[test]
fn set_freezeer_performed_by_non_contract_owner_should_fail() {
    let env = TestEnv::default();
    let non_owner = &env.test_accs[1];

    let err = env
        .tokenfactory_issuer
        .set_freezer(&non_owner.address(), true, non_owner)
        .unwrap_err();

    assert_eq!(
        err,
        TokenfactoryIssuer::execute_error(ContractError::Unauthorized {})
    );
}

#[test]
fn set_freezeer_to_false_should_remove_it_from_state() {
    let env = TestEnv::default();
    let owner = &env.test_accs[0];

    let mut sorted_addrs = env
        .test_accs
        .iter()
        .map(|acc| acc.address())
        .collect::<Vec<_>>();
    sorted_addrs.sort();

    env.tokenfactory_issuer
        .set_freezer(&sorted_addrs[0], true, owner)
        .unwrap();
    env.tokenfactory_issuer
        .set_freezer(&sorted_addrs[1], true, owner)
        .unwrap();

    assert_eq!(
        env.tokenfactory_issuer
            .query_freezer_allowances(None, None)
            .unwrap()
            .freezers,
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

    env.tokenfactory_issuer
        .set_freezer(&sorted_addrs[1], false, owner)
        .unwrap();

    assert_eq!(
        env.tokenfactory_issuer
            .query_freezer_allowances(None, None)
            .unwrap()
            .freezers,
        vec![StatusInfo {
            address: sorted_addrs[0].clone(),
            status: true
        },]
    );

    assert!(
        !env.tokenfactory_issuer
            .query_is_freezer(&sorted_addrs[1])
            .unwrap()
            .status
    );
}

#[test]
fn query_freezer_within_limit() {
    let env = TestEnv::default();
    let owner = &env.test_accs[0];

    let mut sorted_addrs = env
        .test_accs
        .iter()
        .map(|acc| acc.address())
        .collect::<Vec<_>>();
    sorted_addrs.sort();

    // construct allowances and set mint allowance
    let freezers = sorted_addrs
        .iter()
        .map(|addr| StatusInfo {
            address: addr.to_string(),
            status: true,
        })
        .collect::<Vec<_>>();

    freezers.iter().for_each(|allowance| {
        env.tokenfactory_issuer
            .set_freezer(&allowance.address, true, owner)
            .unwrap();
    });

    assert_eq!(
        env.tokenfactory_issuer
            .query_freezer_allowances(None, None)
            .unwrap()
            .freezers,
        freezers
    );

    assert_eq!(
        env.tokenfactory_issuer
            .query_freezer_allowances(None, Some(1))
            .unwrap()
            .freezers,
        freezers[0..1]
    );

    assert_eq!(
        env.tokenfactory_issuer
            .query_freezer_allowances(Some(sorted_addrs[1].clone()), Some(1))
            .unwrap()
            .freezers,
        freezers[2..3]
    );

    assert_eq!(
        env.tokenfactory_issuer
            .query_freezer_allowances(Some(sorted_addrs[1].clone()), Some(10))
            .unwrap()
            .freezers,
        freezers[2..]
    );
}

#[test]
fn query_freezer_off_limit() {
    let env = TestEnv::default();
    let owner = &env.test_accs[0];

    let test_accs_with_allowance =
        TestEnv::create_default_test_accs(&env.tokenfactory_issuer.app, 40);

    let mut sorted_addrs = test_accs_with_allowance
        .iter()
        .map(|acc| acc.address())
        .collect::<Vec<_>>();
    sorted_addrs.sort();

    // construct allowances and set mint allowance
    let freezers = sorted_addrs
        .iter()
        .map(|addr| StatusInfo {
            address: addr.to_string(),
            status: true,
        })
        .collect::<Vec<_>>();

    freezers.iter().for_each(|allowance| {
        env.tokenfactory_issuer
            .set_freezer(&allowance.address, true, owner)
            .unwrap();
    });
    // default limit 10
    assert_eq!(
        env.tokenfactory_issuer
            .query_freezer_allowances(None, None)
            .unwrap()
            .freezers,
        freezers[..10]
    );

    // start after nth, get n+1 .. n+1+limit (10)
    assert_eq!(
        env.tokenfactory_issuer
            .query_freezer_allowances(Some(sorted_addrs[4].clone()), None)
            .unwrap()
            .freezers,
        freezers[5..15]
    );

    // max limit 30
    assert_eq!(
        env.tokenfactory_issuer
            .query_freezer_allowances(None, Some(40))
            .unwrap()
            .freezers,
        freezers[..30]
    );

    // start after nth, get n+1 .. n+1+limit (30)
    assert_eq!(
        env.tokenfactory_issuer
            .query_freezer_allowances(Some(sorted_addrs[4].clone()), Some(40))
            .unwrap()
            .freezers,
        freezers[5..35]
    );
}
