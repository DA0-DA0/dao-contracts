use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::{
    coin, coins, to_binary, Addr, Coin, CosmosMsg, DistributionMsg, Empty, StakingMsg, Uint128,
};
use cw20::{Cw20Coin, Cw20ExecuteMsg};
use cw_denom::{CheckedDenom, UncheckedDenom};
use cw_multi_test::{App, BankSudo, Contract, ContractWrapper, Executor, SudoMsg};
use dao_testing::contracts::cw20_base_contract;
use wynd_utils::Curve;

use crate::contract::{execute, instantiate};
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, ReceiveMsg};
use crate::state::{UncheckedVestingParams, VestingPayment, VestingPaymentStatus};
use crate::ContractError;

const ALICE: &str = "alice";
const BOB: &str = "bob";
const INITIAL_BALANCE: u128 = 1000000000;
const OWNER: &str = "owner";
const NATIVE_DENOM: &str = "ujuno";
const VALIDATOR: &str = "validator";
const VALIDATOR_TWO: &str = "validator2";

fn cw_payroll_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    );
    Box::new(contract)
}

fn get_vesting_payment(app: &App, cw_payroll_addr: Addr) -> VestingPayment {
    app.wrap()
        .query_wasm_smart(cw_payroll_addr, &QueryMsg::Info {})
        .unwrap()
}

fn get_balance_cw20<T: Into<String>, U: Into<String>>(
    app: &App,
    contract_addr: T,
    address: U,
) -> Uint128 {
    let msg = cw20::Cw20QueryMsg::Balance {
        address: address.into(),
    };
    let result: cw20::BalanceResponse = app.wrap().query_wasm_smart(contract_addr, &msg).unwrap();
    result.balance
}

fn get_balance_native<T: Into<String>, U: Into<String>>(
    app: &App,
    address: T,
    denom: U,
) -> Uint128 {
    app.wrap().query_balance(address, denom).unwrap().amount
}

pub fn setup_app() -> App {
    let mut app = App::default();

    // Mint Alice and Bob native tokens
    app.sudo(SudoMsg::Bank({
        BankSudo::Mint {
            to_address: ALICE.to_string(),
            amount: coins(INITIAL_BALANCE, NATIVE_DENOM),
        }
    }))
    .unwrap();
    app.sudo(SudoMsg::Bank({
        BankSudo::Mint {
            to_address: BOB.to_string(),
            amount: coins(INITIAL_BALANCE, NATIVE_DENOM),
        }
    }))
    .unwrap();
    app.sudo(SudoMsg::Bank({
        BankSudo::Mint {
            to_address: OWNER.to_string(),
            amount: coins(INITIAL_BALANCE, NATIVE_DENOM),
        }
    }))
    .unwrap();

    app
}

pub fn setup_contracts(app: &mut App) -> (Addr, u64, u64) {
    let cw20_code_id = app.store_code(cw20_base_contract());
    let cw_payroll_code_id = app.store_code(cw_payroll_contract());

    // Instantiate cw20 contract with balances for Alice and Bob
    let cw20_addr = app
        .instantiate_contract(
            cw20_code_id,
            Addr::unchecked(OWNER),
            &cw20_base::msg::InstantiateMsg {
                name: "cw20 token".to_string(),
                symbol: "cwtwenty".to_string(),
                decimals: 6,
                initial_balances: vec![
                    Cw20Coin {
                        address: ALICE.to_string(),
                        amount: Uint128::new(INITIAL_BALANCE),
                    },
                    Cw20Coin {
                        address: BOB.to_string(),
                        amount: Uint128::new(INITIAL_BALANCE),
                    },
                    Cw20Coin {
                        address: OWNER.to_string(),
                        amount: Uint128::new(INITIAL_BALANCE),
                    },
                ],
                mint: None,
                marketing: None,
            },
            &[],
            "cw20-base",
            None,
        )
        .unwrap();

    (cw20_addr, cw20_code_id, cw_payroll_code_id)
}

struct TestCase {
    cw20_addr: Addr,
    cw_payroll_addr: Addr,
    owner: Addr,
    recipient: Addr,
    vesting_payment: VestingPayment,
}

fn setup_test_case(
    app: &mut App,
    vesting_schedule: Curve,
    amount: Uint128,
    denom: UncheckedDenom,
    recipient: &str,
    owner: Option<String>,
    funds: &[Coin],
) -> TestCase {
    let (cw20_addr, _, cw_payroll_code_id) = setup_contracts(app);

    // Instantiate cw-payroll contract
    let cw_payroll_addr = app
        .instantiate_contract(
            cw_payroll_code_id,
            Addr::unchecked(OWNER),
            &InstantiateMsg {
                owner,
                params: UncheckedVestingParams {
                    recipient: recipient.to_string(),
                    amount,
                    denom: denom.clone(),
                    vesting_schedule,
                    title: None,
                    description: None,
                },
            },
            funds,
            "cw-payroll",
            None,
        )
        .unwrap();

    let vesting_payment = match denom {
        UncheckedDenom::Cw20(ref cw20_addr) => {
            let msg = Cw20ExecuteMsg::Send {
                contract: cw_payroll_addr.to_string(),
                amount,
                msg: to_binary(&ReceiveMsg::Fund {}).unwrap(),
            };
            app.execute_contract(
                Addr::unchecked(OWNER),
                Addr::unchecked(cw20_addr.clone()),
                &msg,
                &[],
            )
            .unwrap();

            get_vesting_payment(app, cw_payroll_addr.clone())
        }
        UncheckedDenom::Native(_) => get_vesting_payment(app, cw_payroll_addr.clone()),
    };

    TestCase {
        cw20_addr,
        cw_payroll_addr,
        owner: Addr::unchecked(OWNER),
        recipient: Addr::unchecked(recipient),
        vesting_payment,
    }
}

#[test]
fn test_happy_cw20_path() {
    let mut app = setup_app();

    let amount = Uint128::new(1000000);
    // cw20 is the first contract instantiated and will hve the address "contract0"
    let unchecked_denom = UncheckedDenom::Cw20("contract0".to_string());

    // Basic linear vesting schedule
    let start_time = app.block_info().time.plus_seconds(100).seconds();
    let end_time = app.block_info().time.plus_seconds(300).seconds();
    let vesting_schedule = Curve::saturating_linear((start_time, amount.into()), (end_time, 0));

    let TestCase {
        cw20_addr,
        cw_payroll_addr,
        recipient: bob,
        vesting_payment,
        ..
    } = setup_test_case(
        &mut app,
        vesting_schedule.clone(),
        amount,
        unchecked_denom,
        BOB,
        None,
        &[],
    );

    // Check Vesting Payment was created correctly
    assert_eq!(
        vesting_payment,
        VestingPayment {
            recipient: Addr::unchecked(BOB),
            amount,
            claimed_amount: Uint128::zero(),
            denom: CheckedDenom::Cw20(Addr::unchecked(cw20_addr.clone())),
            canceled_at_time: None,
            vesting_schedule,
            title: None,
            description: None,
            status: VestingPaymentStatus::Active,
            staked_amount: Uint128::zero(),
        }
    );

    // VestingPayment has not started
    let err: ContractError = app
        .execute_contract(
            bob.clone(),
            cw_payroll_addr.clone(),
            &ExecuteMsg::Distribute {},
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(err, ContractError::NoFundsToClaim);

    // Advance the clock
    app.update_block(|block| {
        block.time = block.time.plus_seconds(150);
    });

    // VestingPayment has started so tokens have vested
    app.execute_contract(
        bob,
        cw_payroll_addr.clone(),
        &ExecuteMsg::Distribute {},
        &[],
    )
    .unwrap();

    // Check final amounts after distribution
    assert_eq!(
        get_vesting_payment(&app, cw_payroll_addr),
        VestingPayment {
            recipient: Addr::unchecked(BOB),
            amount: Uint128::new(750000),
            claimed_amount: Uint128::new(250000),
            ..vesting_payment
        }
    );

    // Owner has funded the contract and down 1000
    assert_eq!(
        get_balance_cw20(&app, cw20_addr.clone(), OWNER),
        Uint128::new(INITIAL_BALANCE) - amount
    );

    // Bob has claimed vested funds and is up 250
    assert_eq!(
        get_balance_cw20(&app, cw20_addr, BOB),
        Uint128::new(INITIAL_BALANCE) + Uint128::new(250000)
    );
}

#[test]
fn test_catch_imposter_cw20() {
    let mut app = setup_app();
    let (cw20_addr, cw20_code_id, _) = setup_contracts(&mut app);

    let amount = Uint128::new(1000000);
    let unchecked_denom = UncheckedDenom::Cw20(cw20_addr.to_string());

    let start_time = app.block_info().time.plus_seconds(100).seconds();
    let end_time = app.block_info().time.plus_seconds(300).seconds();
    let vesting_schedule = Curve::saturating_linear((start_time, amount.into()), (end_time, 0));

    // Instantiate cw-payroll
    let TestCase {
        cw_payroll_addr, ..
    } = setup_test_case(
        &mut app,
        vesting_schedule,
        amount,
        unchecked_denom,
        BOB,
        None,
        &[],
    );

    // Create imposter cw20
    let cw20_imposter_addr = app
        .instantiate_contract(
            cw20_code_id,
            Addr::unchecked(OWNER),
            &cw20_base::msg::InstantiateMsg {
                name: "cw20 token".to_string(),
                symbol: "cwtwenty".to_string(),
                decimals: 6,
                initial_balances: vec![Cw20Coin {
                    address: OWNER.to_string(),
                    amount: Uint128::new(INITIAL_BALANCE),
                }],
                mint: None,
                marketing: None,
            },
            &[],
            "cw20-base",
            None,
        )
        .unwrap();

    let msg = Cw20ExecuteMsg::Send {
        contract: cw_payroll_addr.to_string(),
        amount,
        msg: to_binary(&ReceiveMsg::Fund {}).unwrap(),
    };

    // Errors that cw20 does not match what was expected
    let error: ContractError = app
        .execute_contract(
            Addr::unchecked(OWNER),
            Addr::unchecked(cw20_imposter_addr),
            &msg,
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(error, ContractError::Cw20DoesNotMatch);
}

#[test]
fn test_catch_incorrect_cw20_funding_amount() {
    let mut app = setup_app();
    let (cw20_addr, _, _) = setup_contracts(&mut app);

    let amount = Uint128::new(1000000);
    let unchecked_denom = UncheckedDenom::Cw20(cw20_addr.to_string());

    let start_time = app.block_info().time.plus_seconds(100).seconds();
    let end_time = app.block_info().time.plus_seconds(300).seconds();
    let vesting_schedule = Curve::saturating_linear((start_time, amount.into()), (end_time, 0));

    // Instantiate cw-payroll
    let TestCase {
        cw_payroll_addr, ..
    } = setup_test_case(
        &mut app,
        vesting_schedule,
        amount,
        unchecked_denom,
        BOB,
        None,
        &[],
    );

    let msg = Cw20ExecuteMsg::Send {
        contract: cw_payroll_addr.to_string(),
        amount: Uint128::new(100),
        msg: to_binary(&ReceiveMsg::Fund {}).unwrap(),
    };

    // Errors that cw20 does not match what was expected
    let error: ContractError = app
        .execute_contract(
            Addr::unchecked(OWNER),
            Addr::unchecked(cw20_addr),
            &msg,
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(error, ContractError::AmountDoesNotMatch);
}

#[test]
fn test_happy_native_path() {
    let mut app = setup_app();

    let amount = Uint128::new(1000000);
    let unchecked_denom = UncheckedDenom::Native(NATIVE_DENOM.to_string());

    // Basic linear vesting schedule
    let start_time = app.block_info().time.plus_seconds(100).seconds();
    let end_time = app.block_info().time.plus_seconds(300).seconds();
    let vesting_schedule = Curve::saturating_linear((start_time, amount.into()), (end_time, 0));

    let TestCase {
        cw_payroll_addr,
        recipient: bob,
        vesting_payment,
        ..
    } = setup_test_case(
        &mut app,
        vesting_schedule.clone(),
        amount,
        unchecked_denom,
        BOB,
        None,
        &coins(amount.into(), NATIVE_DENOM),
    );

    // Check Vesting Payment was created correctly
    assert_eq!(
        vesting_payment,
        VestingPayment {
            recipient: Addr::unchecked(BOB),
            amount,
            claimed_amount: Uint128::zero(),
            canceled_at_time: None,
            denom: CheckedDenom::Native(NATIVE_DENOM.to_string()),
            vesting_schedule,
            title: None,
            description: None,
            status: VestingPaymentStatus::Active,
            staked_amount: Uint128::zero(),
        }
    );

    // VestingPayment has not started
    let err: ContractError = app
        .execute_contract(
            bob.clone(),
            cw_payroll_addr.clone(),
            &ExecuteMsg::Distribute {},
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(err, ContractError::NoFundsToClaim);

    // Advance the clock
    app.update_block(|block| {
        block.time = block.time.plus_seconds(150);
    });

    // VestingPayment has started so tokens have vested
    app.execute_contract(
        bob,
        cw_payroll_addr.clone(),
        &ExecuteMsg::Distribute {},
        &[],
    )
    .unwrap();

    // Check final amounts after distribution
    assert_eq!(
        get_vesting_payment(&app, cw_payroll_addr),
        VestingPayment {
            recipient: Addr::unchecked(BOB),
            amount: Uint128::new(750000),
            claimed_amount: Uint128::new(250000),
            ..vesting_payment
        }
    );

    // Owner has funded the contract and down 1000
    assert_eq!(
        get_balance_native(&app, OWNER, NATIVE_DENOM),
        Uint128::new(INITIAL_BALANCE) - amount
    );
    // Bob has claimed vested funds and is up 250
    assert_eq!(
        get_balance_native(&app, BOB, NATIVE_DENOM),
        Uint128::new(INITIAL_BALANCE) + Uint128::new(250000)
    );
}

#[test]
fn test_incorrect_native_funding_amount() {
    let mut app = setup_app();

    let amount = Uint128::new(1000000);
    let unchecked_denom = UncheckedDenom::Native(NATIVE_DENOM.to_string());

    // Basic linear vesting schedule
    let start_time = app.block_info().time.plus_seconds(100).seconds();
    let end_time = app.block_info().time.plus_seconds(300).seconds();
    let vesting_schedule = Curve::saturating_linear((start_time, amount.into()), (end_time, 0));

    let alice = Addr::unchecked(ALICE);

    let (_, _, cw_payroll_code_id) = setup_contracts(&mut app);

    // Instantiate cw-payroll contract errors with incorrect amount
    let error: ContractError = app
        .instantiate_contract(
            cw_payroll_code_id,
            alice.clone(),
            &InstantiateMsg {
                owner: Some(alice.to_string()),
                params: UncheckedVestingParams {
                    recipient: BOB.to_string(),
                    amount,
                    denom: unchecked_denom,
                    vesting_schedule,
                    title: None,
                    description: None,
                },
            },
            &coins(100, NATIVE_DENOM),
            "cw-payroll",
            None,
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(error, ContractError::AmountDoesNotMatch)
}

#[test]
fn test_cancel_vesting() {
    let mut app = setup_app();

    let amount = Uint128::new(1000000);
    let unchecked_denom = UncheckedDenom::Native(NATIVE_DENOM.to_string());

    // Basic linear vesting schedule
    let start_time = app.block_info().time.plus_seconds(100).seconds();
    let end_time = app.block_info().time.plus_seconds(300).seconds();
    let vesting_schedule = Curve::saturating_linear((start_time, amount.into()), (end_time, 0));

    let alice = Addr::unchecked(ALICE);

    let TestCase {
        cw_payroll_addr,
        owner,
        recipient: bob,
        ..
    } = setup_test_case(
        &mut app,
        vesting_schedule,
        amount,
        unchecked_denom,
        BOB,
        Some(OWNER.to_string()),
        &coins(amount.into(), NATIVE_DENOM),
    );

    // Non-owner can't cancel
    let err: ContractError = app
        .execute_contract(alice, cw_payroll_addr.clone(), &ExecuteMsg::Cancel {}, &[])
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(
        err,
        ContractError::Ownable(cw_ownable::OwnershipError::NotOwner)
    );

    // Advance the clock
    app.update_block(|block| {
        block.time = block.time.plus_seconds(150);
    });

    // Owner DAO cancels vesting contract
    app.execute_contract(owner, cw_payroll_addr.clone(), &ExecuteMsg::Cancel {}, &[])
        .unwrap();

    // Bob tries to withdraw but can't
    app.execute_contract(bob, cw_payroll_addr, &ExecuteMsg::Distribute {}, &[])
        .unwrap_err();

    // Unvested funds have been returned to contract owner
    assert_eq!(
        get_balance_native(&app, "owner", NATIVE_DENOM),
        Uint128::new(INITIAL_BALANCE) - Uint128::new(250000)
    );
    // Bob has gets the funds vest up until cancelation
    assert_eq!(
        get_balance_native(&app, BOB, NATIVE_DENOM),
        Uint128::new(INITIAL_BALANCE) + Uint128::new(250000)
    );
}

#[test]
fn staking_unit_tests() {
    let mut deps = mock_dependencies();

    // Update staking querier info
    deps.querier.update_staking(NATIVE_DENOM, &[], &[]);

    let env = mock_env();
    let alice = mock_info(ALICE, &[]);
    let bob = mock_info(BOB, &[]);

    let amount = Uint128::new(1000000);
    let unchecked_denom = UncheckedDenom::Native(NATIVE_DENOM.to_string());

    // Basic linear vesting schedule
    let start_time = env.block.time.plus_seconds(100).seconds();
    let end_time = env.block.time.plus_seconds(300).seconds();
    let vesting_schedule = Curve::saturating_linear((start_time, amount.into()), (end_time, 0));

    // Alice successfully instantiates
    instantiate(
        deps.as_mut(),
        env.clone(),
        mock_info(ALICE, &coins(amount.into(), NATIVE_DENOM.to_string())),
        InstantiateMsg {
            owner: Some(OWNER.to_string()),
            params: UncheckedVestingParams {
                recipient: BOB.to_string(),
                amount,
                denom: unchecked_denom,
                vesting_schedule,
                title: None,
                description: None,
            },
        },
    )
    .unwrap();

    // Alice can't delegate his vesting payment
    let err = execute(
        deps.as_mut(),
        env.clone(),
        alice.clone(),
        ExecuteMsg::Delegate {
            validator: VALIDATOR.to_string(),
            amount,
        },
    )
    .unwrap_err();
    assert_eq!(err, ContractError::Unauthorized);

    // Bob delegates his vesting payment
    let res = execute(
        deps.as_mut(),
        env.clone(),
        bob.clone(),
        ExecuteMsg::Delegate {
            validator: VALIDATOR.to_string(),
            amount,
        },
    )
    .unwrap();
    assert_eq!(
        res.messages[0].msg,
        CosmosMsg::Staking(StakingMsg::Delegate {
            validator: VALIDATOR.to_string(),
            amount: coin(amount.into(), NATIVE_DENOM)
        })
    );

    // Bob can't delegate more than he has
    execute(
        deps.as_mut(),
        env.clone(),
        bob.clone(),
        ExecuteMsg::Delegate {
            validator: VALIDATOR.to_string(),
            amount,
        },
    )
    .unwrap_err();

    // Any can call Withdraw Rewards, even alice
    let res = execute(
        deps.as_mut(),
        env.clone(),
        alice.clone(),
        ExecuteMsg::WithdrawDelegatorReward {
            validator: VALIDATOR.to_string(),
        },
    )
    .unwrap();
    assert_eq!(
        res.messages[0].msg,
        CosmosMsg::Distribution(DistributionMsg::WithdrawDelegatorReward {
            validator: VALIDATOR.to_string(),
        })
    );

    // Alice can't redelegate or undelegate on bob's behalf
    let err = execute(
        deps.as_mut(),
        env.clone(),
        alice.clone(),
        ExecuteMsg::Redelegate {
            src_validator: VALIDATOR.to_string(),
            dst_validator: VALIDATOR_TWO.to_string(),
            amount,
        },
    )
    .unwrap_err();
    assert_eq!(err, ContractError::Unauthorized);
    let err = execute(
        deps.as_mut(),
        env.clone(),
        alice.clone(),
        ExecuteMsg::Undelegate {
            validator: VALIDATOR.to_string(),
            amount,
        },
    )
    .unwrap_err();
    assert_eq!(err, ContractError::Unauthorized);

    // Bob (recipient) can't redelegate more than they have
    execute(
        deps.as_mut(),
        env.clone(),
        bob.clone(),
        ExecuteMsg::Redelegate {
            src_validator: VALIDATOR.to_string(),
            dst_validator: VALIDATOR_TWO.to_string(),
            amount: amount + Uint128::new(1000),
        },
    )
    .unwrap_err();

    // Bob redelegates half their tokens
    let res = execute(
        deps.as_mut(),
        env.clone(),
        bob.clone(),
        ExecuteMsg::Redelegate {
            src_validator: VALIDATOR.to_string(),
            dst_validator: VALIDATOR_TWO.to_string(),
            amount: amount - amount.checked_div(Uint128::new(2)).unwrap(),
        },
    )
    .unwrap();
    assert_eq!(
        res.messages[0].msg,
        CosmosMsg::Staking(StakingMsg::Redelegate {
            src_validator: VALIDATOR.to_string(),
            dst_validator: VALIDATOR_TWO.to_string(),
            amount: Coin {
                denom: NATIVE_DENOM.to_string(),
                amount: amount - amount.checked_div(Uint128::new(2)).unwrap(),
            }
        })
    );

    // Bob undelegates a little from validator two
    let res = execute(
        deps.as_mut(),
        env.clone(),
        bob.clone(),
        ExecuteMsg::Undelegate {
            validator: VALIDATOR_TWO.to_string(),
            amount: Uint128::new(10),
        },
    )
    .unwrap();
    assert_eq!(
        res.messages[0].msg,
        CosmosMsg::Staking(StakingMsg::Undelegate {
            validator: VALIDATOR_TWO.to_string(),
            amount: coin(10, NATIVE_DENOM)
        })
    );

    // Only Bob (the recipient) can call SetWithdrawAddress
    let err = execute(
        deps.as_mut(),
        env.clone(),
        alice,
        ExecuteMsg::SetWithdrawAddress {
            address: ALICE.to_string(),
        },
    )
    .unwrap_err();
    assert_eq!(err, ContractError::Unauthorized);
    let res = execute(
        deps.as_mut(),
        env.clone(),
        bob,
        ExecuteMsg::SetWithdrawAddress {
            address: "bob2".to_string(),
        },
    )
    .unwrap();
    assert_eq!(
        res.messages[0].msg,
        CosmosMsg::Distribution(DistributionMsg::SetWithdrawAddress {
            address: "bob2".to_string()
        })
    );

    // Contract owner cancels contract, it includes unbonding message for all validators bob delegates to
    let res = execute(
        deps.as_mut(),
        env,
        mock_info(OWNER, &[]),
        ExecuteMsg::Cancel {},
    )
    .unwrap();
    assert_eq!(res.messages.len(), 3);
    assert_eq!(
        res.messages[1].msg,
        CosmosMsg::Staking(StakingMsg::Undelegate {
            validator: VALIDATOR.to_string(),
            amount: coin(
                amount.checked_div(Uint128::new(2)).unwrap().into(),
                NATIVE_DENOM
            )
        })
    );
    assert_eq!(
        res.messages[2].msg,
        CosmosMsg::Staking(StakingMsg::Undelegate {
            validator: VALIDATOR_TWO.to_string(),
            amount: coin(
                amount
                    .checked_div(Uint128::new(2))
                    .unwrap()
                    .checked_sub(Uint128::new(10))
                    .unwrap()
                    .into(),
                NATIVE_DENOM
            )
        })
    );
}
