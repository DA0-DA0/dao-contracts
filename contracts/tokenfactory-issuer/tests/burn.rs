mod helpers;
use cosmwasm_std::{OverflowError, OverflowOperation, StdError, Uint128};
use helpers::{TestEnv, TokenfactoryIssuer};
use osmosis_testing::{
    cosmrs::proto::cosmos::bank::v1beta1::QueryBalanceRequest, Account, RunnerError,
};
use tokenfactory_issuer::{msg::AllowanceInfo, ContractError};

#[test]
fn set_burner_performed_by_contract_owner_should_pass() {
    let env = TestEnv::default();
    let owner = &env.test_accs[0];
    let non_owner = &env.test_accs[1];

    let allowance = 1000000;
    env.tokenfactory_issuer
        .set_burner(&non_owner.address(), allowance, owner)
        .unwrap();

    let burn_allowance = env
        .tokenfactory_issuer
        .query_burn_allowance(&env.test_accs[1].address())
        .unwrap()
        .allowance;

    assert_eq!(burn_allowance.u128(), allowance);
}

#[test]
fn set_burner_performed_by_non_contract_owner_should_fail() {
    let env = TestEnv::default();
    let non_owner = &env.test_accs[1];

    let allowance = 1000000;

    let err = env
        .tokenfactory_issuer
        .set_burner(&non_owner.address(), allowance, non_owner)
        .unwrap_err();

    assert_eq!(
        err,
        TokenfactoryIssuer::execute_error(ContractError::Unauthorized {})
    );
}

#[test]
fn burn_whole_balance_but_less_than_or_eq_allowance_should_pass() {
    let cases = vec![
        (u128::MAX, u128::MAX),
        (u128::MAX, u128::MAX - 1),
        (u128::MAX, 1),
        (2, 1),
        (1, 1),
    ];

    cases.into_iter().for_each(|(allowance, burn_amount)| {
        let env = TestEnv::default();
        let owner = &env.test_accs[0];
        let denom = env.tokenfactory_issuer.query_denom().unwrap().denom;

        let burner = &env.test_accs[1];
        let burn_to = &env.test_accs[2];

        // mint
        env.tokenfactory_issuer
            .set_minter(&burner.address(), allowance, owner)
            .unwrap();

        env.tokenfactory_issuer
            .mint(&burn_to.address(), burn_amount, burner)
            .unwrap();

        // burn
        env.tokenfactory_issuer
            .set_burner(&burner.address(), allowance, owner)
            .unwrap();

        env.tokenfactory_issuer
            .burn(&burn_to.address(), burn_amount, burner)
            .unwrap();

        let amount = env
            .bank()
            .query_balance(&QueryBalanceRequest {
                address: burn_to.address(),
                denom,
            })
            .unwrap()
            .balance
            .unwrap()
            .amount;

        assert_eq!(amount, "0");
    });
}

#[test]
fn burn_more_than_balance_should_fail() {
    let cases = vec![(u128::MAX - 1, u128::MAX), (1, 2)];

    cases.into_iter().for_each(|(balance, burn_amount)| {
        let env = TestEnv::default();
        let owner = &env.test_accs[0];
        let denom = env.tokenfactory_issuer.query_denom().unwrap().denom;

        let burner = &env.test_accs[1];
        let burn_from = &env.test_accs[2];

        // mint
        env.tokenfactory_issuer
            .set_minter(&burner.address(), balance, owner)
            .unwrap();

        env.tokenfactory_issuer
            .mint(&burn_from.address(), balance, burner)
            .unwrap();

        // burn
        env.tokenfactory_issuer
            .set_burner(&burner.address(), burn_amount, owner)
            .unwrap();

        let err = env
            .tokenfactory_issuer
            .burn(&burn_from.address(), burn_amount, burner)
            .unwrap_err();

        assert_eq!(
            err,
            RunnerError::ExecuteError {
                msg: format!("failed to execute message; message index: 0: dispatch: submessages: {balance}{denom} is smaller than {burn_amount}{denom}: insufficient funds")
            }
        );
    });
}

#[test]
fn burn_over_allowance_should_fail() {
    let cases = vec![(u128::MAX - 1, u128::MAX), (0, 1)];

    cases.into_iter().for_each(|(allowance, burn_amount)| {
        let env = TestEnv::default();
        let owner = &env.test_accs[0];

        let burner = &env.test_accs[1];
        let burn_from = &env.test_accs[2];

        env.tokenfactory_issuer
            .set_burner(&burner.address(), allowance, owner)
            .unwrap();

        let err = env
            .tokenfactory_issuer
            .burn(&burn_from.address(), burn_amount, burner)
            .unwrap_err();

        assert_eq!(
            err,
            TokenfactoryIssuer::execute_error(ContractError::Std(StdError::Overflow {
                source: OverflowError {
                    operation: OverflowOperation::Sub,
                    operand1: allowance.to_string(),
                    operand2: burn_amount.to_string(),
                }
            }))
        );
    });
}

#[test]
fn burn_0_should_fail() {
    let cases = vec![(u128::MAX, 0), (0, 0)];

    cases.into_iter().for_each(|(allowance, burn_amount)| {
        let env = TestEnv::default();
        let owner = &env.test_accs[0];

        let burner = &env.test_accs[1];
        let burn_to = &env.test_accs[2];

        env.tokenfactory_issuer
            .set_burner(&burner.address(), allowance, owner)
            .unwrap();

        let err = env
            .tokenfactory_issuer
            .burn(&burn_to.address(), burn_amount, burner)
            .unwrap_err();

        assert_eq!(
            err,
            TokenfactoryIssuer::execute_error(ContractError::ZeroAmount {})
        );
    });
}

#[test]
fn query_burn_allowances_within_limit() {
    let env = TestEnv::default();
    let owner = &env.test_accs[0];

    let mut sorted_addrs = env
        .test_accs
        .iter()
        .map(|acc| acc.address())
        .collect::<Vec<_>>();
    sorted_addrs.sort();

    // construct allowances and set burn allowance
    let allowances = sorted_addrs
        .iter()
        .enumerate()
        .map(|(i, addr)| AllowanceInfo {
            address: addr.to_string(),
            allowance: Uint128::from((i as u128 + 1) * 10000u128),
        })
        .collect::<Vec<_>>();

    allowances.iter().for_each(|allowance| {
        env.tokenfactory_issuer
            .set_burner(&allowance.address, allowance.allowance.u128(), owner)
            .unwrap();
    });

    assert_eq!(
        env.tokenfactory_issuer
            .query_burn_allowances(None, None)
            .unwrap()
            .allowances,
        allowances
    );

    assert_eq!(
        env.tokenfactory_issuer
            .query_burn_allowances(None, Some(1))
            .unwrap()
            .allowances,
        allowances[0..1]
    );

    assert_eq!(
        env.tokenfactory_issuer
            .query_burn_allowances(Some(sorted_addrs[1].clone()), Some(1))
            .unwrap()
            .allowances,
        allowances[2..3]
    );

    assert_eq!(
        env.tokenfactory_issuer
            .query_burn_allowances(Some(sorted_addrs[1].clone()), Some(10))
            .unwrap()
            .allowances,
        allowances[2..]
    );
}

#[test]
fn query_burn_allowance_off_limit() {
    let env = TestEnv::default();
    let owner = &env.test_accs[0];

    let test_accs_with_allowance =
        TestEnv::create_default_test_accs(&env.tokenfactory_issuer.app, 40);

    let mut sorted_addrs = test_accs_with_allowance
        .iter()
        .map(|acc| acc.address())
        .collect::<Vec<_>>();
    sorted_addrs.sort();

    // construct allowances and set burn allowance
    let allowances = sorted_addrs
        .iter()
        .enumerate()
        .map(|(i, addr)| AllowanceInfo {
            address: addr.to_string(),
            allowance: Uint128::from(i as u128 * 10000u128),
        })
        .collect::<Vec<_>>();

    allowances.iter().for_each(|allowance| {
        env.tokenfactory_issuer
            .set_burner(&allowance.address, allowance.allowance.u128(), owner)
            .unwrap();
    });

    // default limit 10
    assert_eq!(
        env.tokenfactory_issuer
            .query_burn_allowances(None, None)
            .unwrap()
            .allowances,
        allowances[..10]
    );

    // start after nth, get n+1 .. n+1+limit (10)
    assert_eq!(
        env.tokenfactory_issuer
            .query_burn_allowances(Some(sorted_addrs[4].clone()), None)
            .unwrap()
            .allowances,
        allowances[5..15]
    );

    // max limit 30
    assert_eq!(
        env.tokenfactory_issuer
            .query_burn_allowances(None, Some(40))
            .unwrap()
            .allowances,
        allowances[..30]
    );

    // start after nth, get n+1 .. n+1+limit (30)
    assert_eq!(
        env.tokenfactory_issuer
            .query_burn_allowances(Some(sorted_addrs[4].clone()), Some(40))
            .unwrap()
            .allowances,
        allowances[5..35]
    );
}
