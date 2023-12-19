use cosmwasm_std::Uint128;
use cw_tokenfactory_issuer::{msg::AllowanceInfo, ContractError};
use osmosis_test_tube::{
    osmosis_std::types::cosmos::bank::v1beta1::QueryBalanceRequest, Account, RunnerError,
};

use crate::test_env::{
    test_query_over_default_limit, test_query_within_default_limit, TestEnv, TokenfactoryIssuer,
};

#[test]
fn set_minter_performed_by_contract_owner_should_pass() {
    let env = TestEnv::default();
    let owner = &env.test_accs[0];
    let non_owner = &env.test_accs[1];

    let allowance = 1000000;
    env.cw_tokenfactory_issuer
        .set_minter(&non_owner.address(), allowance, owner)
        .unwrap();

    let mint_allowance = env
        .cw_tokenfactory_issuer
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
        .cw_tokenfactory_issuer
        .set_minter(&non_owner.address(), allowance, non_owner)
        .unwrap_err();

    assert_eq!(
        err,
        TokenfactoryIssuer::execute_error(ContractError::Ownership(
            cw_ownable::OwnershipError::NotOwner
        ))
    );
}

#[test]
fn set_allowance_to_0_should_remove_it_from_storage() {
    let env = TestEnv::default();
    let owner = &env.test_accs[0];
    let minter = &env.test_accs[1];

    // Set allowance to some value
    let allowance = 1000000;
    env.cw_tokenfactory_issuer
        .set_minter(&minter.address(), allowance, owner)
        .unwrap();

    // Set allowance to 0
    env.cw_tokenfactory_issuer
        .set_minter(&minter.address(), 0, owner)
        .unwrap();

    // Check if key for the minter address is removed
    assert_eq!(
        env.cw_tokenfactory_issuer
            .query_mint_allowances(None, None)
            .unwrap()
            .allowances,
        vec![]
    );
}

#[test]
fn used_up_allowance_should_be_removed_from_storage() {
    let env = TestEnv::default();
    let owner = &env.test_accs[0];
    let minter = &env.test_accs[1];

    // Set allowance to some value
    let allowance = 1000000;
    env.cw_tokenfactory_issuer
        .set_minter(&minter.address(), allowance, owner)
        .unwrap();

    // Use all allowance
    env.cw_tokenfactory_issuer
        .mint(&minter.address(), allowance, minter)
        .unwrap();

    // Check if key for the minter address is removed
    assert_eq!(
        env.cw_tokenfactory_issuer
            .query_mint_allowances(None, None)
            .unwrap()
            .allowances,
        vec![]
    );
}

#[cfg(feature = "osmosis_tokenfactory")]
#[test]
fn granular_minting_permissions_when_frozen() {
    let env = TestEnv::default();
    let owner = &env.test_accs[0];
    let denom = env.cw_tokenfactory_issuer.query_denom().unwrap().denom;

    let minter = &env.test_accs[1];
    let minter_two = &env.test_accs[2];
    let mint_to = &env.test_accs[3];

    // Owner sets before send hook to enable advanced features
    env.cw_tokenfactory_issuer
        .set_before_send_hook(env.cw_tokenfactory_issuer.contract_addr.clone(), owner)
        .unwrap();

    // Owner freezes contract
    env.cw_tokenfactory_issuer.freeze(true, owner).unwrap();

    // Owner grants minter a mint allowance
    env.cw_tokenfactory_issuer
        .set_minter(&minter.address(), 1000000, owner)
        .unwrap();

    // Owner grants minter_two a mint allowance
    env.cw_tokenfactory_issuer
        .set_minter(&minter_two.address(), 1000000, owner)
        .unwrap();

    // Minter can't mint when frozen
    let err = env
        .cw_tokenfactory_issuer
        .mint(&mint_to.address(), 100, minter)
        .unwrap_err();
    assert_eq!(
        err,
        RunnerError::ExecuteError {
            msg: format!("failed to execute message; message index: 0: The contract is frozen for denom \"{}\". Addresses need to be added to the allowlist to enable transfers to or from an account.: execute wasm contract failed", denom)
        }
    );

    // Own puts minter on allowlist
    env.cw_tokenfactory_issuer
        .allow(&minter.address(), true, owner)
        .unwrap();

    // Minter can mint
    env.cw_tokenfactory_issuer
        .mint(&mint_to.address(), 100, minter)
        .unwrap();

    // Minter-two can't mint because not allowed
    let err = env
        .cw_tokenfactory_issuer
        .mint(&mint_to.address(), 100, minter_two)
        .unwrap_err();
    assert_eq!(
        err,
        RunnerError::ExecuteError {
            msg: format!("failed to execute message; message index: 0: The contract is frozen for denom \"{}\". Addresses need to be added to the allowlist to enable transfers to or from an account.: execute wasm contract failed", denom)
        }
    );
}

#[test]
fn mint_less_than_or_eq_allowance_should_pass_and_deduct_allowance() {
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
        let denom = env.cw_tokenfactory_issuer.query_denom().unwrap().denom;

        let minter = &env.test_accs[1];
        let mint_to = &env.test_accs[2];

        env.cw_tokenfactory_issuer
            .set_minter(&minter.address(), allowance, owner)
            .unwrap();

        env.cw_tokenfactory_issuer
            .mint(&mint_to.address(), mint_amount, minter)
            .unwrap();

        // Check if allowance is deducted properly
        let resulted_allowance = env
            .cw_tokenfactory_issuer
            .query_mint_allowance(&minter.address())
            .unwrap()
            .allowance
            .u128();

        assert_eq!(resulted_allowance, allowance - mint_amount);

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
fn mint_over_allowance_should_fail_and_not_deduct_allowance() {
    let cases = vec![(u128::MAX - 1, u128::MAX), (0, 1)];

    cases.into_iter().for_each(|(allowance, mint_amount)| {
        let env = TestEnv::default();
        let owner = &env.test_accs[0];

        let minter = &env.test_accs[1];
        let mint_to = &env.test_accs[2];

        env.cw_tokenfactory_issuer
            .set_minter(&minter.address(), allowance, owner)
            .unwrap();

        let err = env
            .cw_tokenfactory_issuer
            .mint(&mint_to.address(), mint_amount, minter)
            .unwrap_err();

        assert_eq!(
            err,
            TokenfactoryIssuer::execute_error(ContractError::not_enough_mint_allowance(
                mint_amount,
                allowance
            ))
        );

        // Check if allowance stays the same
        let resulted_allowance = env
            .cw_tokenfactory_issuer
            .query_mint_allowance(&minter.address())
            .unwrap()
            .allowance
            .u128();

        assert_eq!(resulted_allowance, allowance);
    });
}

#[test]
fn mint_0_should_fail_and_not_deduct_allowance() {
    let cases = vec![(u128::MAX, 0), (0, 0)];

    cases.into_iter().for_each(|(allowance, mint_amount)| {
        let env = TestEnv::default();
        let owner = &env.test_accs[0];

        let minter = &env.test_accs[1];
        let mint_to = &env.test_accs[2];

        env.cw_tokenfactory_issuer
            .set_minter(&minter.address(), allowance, owner)
            .unwrap();

        let err = env
            .cw_tokenfactory_issuer
            .mint(&mint_to.address(), mint_amount, minter)
            .unwrap_err();

        assert_eq!(
            err,
            TokenfactoryIssuer::execute_error(ContractError::ZeroAmount {})
        );

        // Check if allowance stays the same
        let resulted_allowance = env
            .cw_tokenfactory_issuer
            .query_mint_allowance(&minter.address())
            .unwrap()
            .allowance
            .u128();

        assert_eq!(resulted_allowance, allowance);
    });
}

#[test]
fn test_query_mint_allowances_within_default_limit() {
    test_query_within_default_limit::<AllowanceInfo, _, _>(
        |(i, addr)| AllowanceInfo {
            address: addr.to_string(),
            allowance: Uint128::from((i as u128 + 1) * 10000u128), // generate distincted allowance
        },
        |env| {
            move |allowance| {
                let owner = &env.test_accs[0];
                env.cw_tokenfactory_issuer
                    .set_minter(&allowance.address, allowance.allowance.u128(), owner)
                    .unwrap();
            }
        },
        |env| {
            move |start_after, limit| {
                env.cw_tokenfactory_issuer
                    .query_mint_allowances(start_after, limit)
                    .unwrap()
                    .allowances
            }
        },
    );
}

#[test]
fn test_query_mint_allowance_over_default_limit() {
    test_query_over_default_limit::<AllowanceInfo, _, _>(
        |(i, addr)| AllowanceInfo {
            address: addr.to_string(),
            allowance: Uint128::from((i as u128 + 1) * 10000u128), // generate distincted allowance
        },
        |env| {
            move |allowance| {
                let owner = &env.test_accs[0];
                env.cw_tokenfactory_issuer
                    .set_minter(&allowance.address, allowance.allowance.u128(), owner)
                    .unwrap();
            }
        },
        |env| {
            move |start_after, limit| {
                env.cw_tokenfactory_issuer
                    .query_mint_allowances(start_after, limit)
                    .unwrap()
                    .allowances
            }
        },
    );
}
