mod helpers;
use cosmwasm_std::{OverflowError, OverflowOperation, StdError};
use helpers::{TestEnv, TokenfactoryIssuer};
use osmosis_testing::{cosmrs::proto::cosmos::bank::v1beta1::QueryBalanceRequest, Account};
use tokenfactory_issuer::ContractError;

#[test]
fn set_minter_performed_by_contract_owner_should_pass() {
    let env = TestEnv::default();
    let owner = &env.test_accs[0];
    let non_owner = &env.test_accs[1];

    let allowance = 1000000;
    env.tokenfactory_issuer
        .set_minter(&non_owner.address(), allowance, owner)
        .unwrap();

    let mint_allowance = env
        .tokenfactory_issuer
        .query_mint_allowance(&env.test_accs[1].address())
        .unwrap()
        .allowance;

    assert_eq!(mint_allowance.u128(), allowance);
}

#[test]
fn set_minter_performed_by_non_contract_owner_should_fail() {
    let env = TestEnv::default();
    let non_owner = &env.test_accs[1];

    let allowance = 1000000;

    let err = env
        .tokenfactory_issuer
        .set_minter(&non_owner.address(), allowance, non_owner)
        .unwrap_err();

    assert_eq!(
        err,
        TokenfactoryIssuer::execute_error(ContractError::Unauthorized {})
    );
}

#[test]
fn mint_less_than_or_eq_allowance_should_pass() {
    let cases = vec![
        (u128::MAX, u128::MAX),
        (u128::MAX, u128::MAX - 1),
        (u128::MAX, 1),
        (2, 1),
        (1, 1),
    ];

    cases.into_iter().for_each(|(allowance, mint_amount)| {
        let env = TestEnv::default();
        let owner = &env.test_accs[0];
        let denom = env.tokenfactory_issuer.query_denom().unwrap().denom;

        let minter = &env.test_accs[1];
        let mint_to = &env.test_accs[2];

        env.tokenfactory_issuer
            .set_minter(&minter.address(), allowance, owner)
            .unwrap();

        env.tokenfactory_issuer
            .mint(&mint_to.address(), mint_amount, minter)
            .unwrap();

        let amount = env
            .bank()
            .query_balance(&QueryBalanceRequest {
                address: mint_to.address(),
                denom,
            })
            .unwrap()
            .balance
            .unwrap()
            .amount;

        assert_eq!(amount, mint_amount.to_string());
    });
}

#[test]
fn mint_over_allowance_should_fail() {
    let cases = vec![(u128::MAX - 1, u128::MAX), (0, 1)];

    cases.into_iter().for_each(|(allowance, mint_amount)| {
        let env = TestEnv::default();
        let owner = &env.test_accs[0];

        let minter = &env.test_accs[1];
        let mint_to = &env.test_accs[2];

        env.tokenfactory_issuer
            .set_minter(&minter.address(), allowance, owner)
            .unwrap();

        let err = env
            .tokenfactory_issuer
            .mint(&mint_to.address(), mint_amount, minter)
            .unwrap_err();

        assert_eq!(
            err,
            TokenfactoryIssuer::execute_error(ContractError::Std(StdError::Overflow {
                source: OverflowError {
                    operation: OverflowOperation::Sub,
                    operand1: allowance.to_string(),
                    operand2: mint_amount.to_string(),
                }
            }))
        );
    });
}

#[test]
fn mint_0_should_fail() {
    let cases = vec![(u128::MAX, 0), (0, 0)];

    cases.into_iter().for_each(|(allowance, mint_amount)| {
        let env = TestEnv::default();
        let owner = &env.test_accs[0];

        let minter = &env.test_accs[1];
        let mint_to = &env.test_accs[2];

        env.tokenfactory_issuer
            .set_minter(&minter.address(), allowance, owner)
            .unwrap();

        let err = env
            .tokenfactory_issuer
            .mint(&mint_to.address(), mint_amount, minter)
            .unwrap_err();

        assert_eq!(
            err,
            TokenfactoryIssuer::execute_error(ContractError::ZeroAmount {})
        );
    });
}
