use cosmwasm_std::testing::mock_env;
use cosmwasm_std::{coins, to_binary, Addr, Coin, Decimal, Empty, Uint128, Validator};
use cw20::{Cw20Coin, Cw20ExecuteMsg};
use cw_denom::{CheckedDenom, UncheckedDenom};
use cw_multi_test::{
    App, AppBuilder, BankSudo, Contract, ContractWrapper, Executor, StakingInfo, StakingSudo,
    SudoMsg,
};
use dao_testing::contracts::cw20_base_contract;
use wynd_utils::Curve;

use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, ReceiveMsg};
use crate::state::{
    UncheckedVestingParams, VestingPayment, VestingPaymentRewards, VestingPaymentStatus,
};
use crate::ContractError;

const ALICE: &str = "alice";
const BOB: &str = "bob";
const INITIAL_BALANCE: u128 = 1000000000;
const OWNER: &str = "owner";
const NATIVE_DENOM: &str = "denom";
const VALIDATOR: &str = "validator";

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
    let mut app = AppBuilder::new().build(|router, api, storage| {
        let env = mock_env();

        // Setup staking module for the correct mock data.
        router
            .staking
            .setup(
                storage,
                StakingInfo {
                    bonded_denom: NATIVE_DENOM.to_string(),
                    unbonding_time: 1,
                    apr: Decimal::percent(20),
                },
            )
            .unwrap();

        // Add mock validator
        router
            .staking
            .add_validator(
                api,
                storage,
                &env.block,
                Validator {
                    address: VALIDATOR.to_string(),
                    commission: Decimal::zero(),
                    max_commission: Decimal::one(),
                    max_change_rate: Decimal::one(),
                },
            )
            .unwrap();
    });

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
            rewards: VestingPaymentRewards {
                pending: Decimal::zero(),
                paid_rewards_per_token: Decimal::zero(),
            },
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
            rewards: VestingPaymentRewards {
                pending: Decimal::zero(),
                paid_rewards_per_token: Decimal::zero(),
            },
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
fn test_native_staking_happy_path() {
    let mut app = setup_app();

    let amount = Uint128::new(1000000);
    let unchecked_denom = UncheckedDenom::Native(NATIVE_DENOM.to_string());

    // Basic linear vesting schedule
    let start_time = app.block_info().time.plus_seconds(100).seconds();
    let end_time = app.block_info().time.plus_seconds(300).seconds();
    let vesting_schedule = Curve::saturating_linear((start_time, amount.into()), (end_time, 0));

    // Make vesting payment to bob
    let TestCase {
        cw_payroll_addr,
        recipient: bob,
        vesting_payment,
        ..
    } = setup_test_case(
        &mut app,
        vesting_schedule,
        amount,
        unchecked_denom,
        BOB,
        None,
        &coins(amount.into(), NATIVE_DENOM),
    );

    // Bob delegates his vesting tokens
    app.execute_contract(
        bob.clone(),
        cw_payroll_addr.clone(),
        &ExecuteMsg::Delegate {
            validator: VALIDATOR.to_string(),
            amount,
        },
        &[],
    )
    .unwrap();

    // Bob can't delegate more than his vesting amount
    app.execute_contract(
        bob.clone(),
        cw_payroll_addr.clone(),
        &ExecuteMsg::Delegate {
            validator: VALIDATOR.to_string(),
            amount,
        },
        &[],
    )
    .unwrap_err();

    // Distribute fails because tokens are locked
    app.execute_contract(
        bob.clone(),
        cw_payroll_addr.clone(),
        &ExecuteMsg::Distribute {},
        &[],
    )
    .unwrap_err();

    // Advance the clock
    app.update_block(|block| {
        block.height += 10000;
        block.time = block.time.plus_seconds(10000000);
    });

    // Call withdraw rewards
    app.execute_contract(
        bob.clone(),
        cw_payroll_addr.clone(),
        &ExecuteMsg::WithdrawDelegatorReward {
            validator: VALIDATOR.to_string(),
        },
        &[],
    )
    .unwrap();

    // Bob undelegates
    app.execute_contract(
        bob.clone(),
        cw_payroll_addr.clone(),
        &ExecuteMsg::Undelegate {
            validator: VALIDATOR.to_string(),
            amount,
        },
        &[],
    )
    .unwrap();

    // Advance the clock
    app.update_block(|block| {
        block.height += 10000;
        block.time = block.time.plus_seconds(10000000);
    });

    // Trigger unboding que to return tokens
    app.sudo(SudoMsg::Staking(StakingSudo::ProcessQueue {}))
        .unwrap();

    // Bob distributes remaining funds
    app.execute_contract(
        bob,
        cw_payroll_addr.clone(),
        &ExecuteMsg::Distribute {},
        &[],
    )
    .unwrap();

    // Bob has claimed vested funds and staked funds
    assert_eq!(
        get_balance_native(&app, BOB, NATIVE_DENOM),
        Uint128::new(1001063419)
    );

    // Check vesting payment status after final distribution
    assert_eq!(
        get_vesting_payment(&app, cw_payroll_addr),
        VestingPayment {
            recipient: Addr::unchecked(BOB),
            amount: Uint128::new(0),
            claimed_amount: Uint128::new(1000000),
            rewards: VestingPaymentRewards {
                pending: Decimal::zero(),
                paid_rewards_per_token: Decimal::new(Uint128::new(63419000000000000))
            },
            status: VestingPaymentStatus::FullyVested,
            ..vesting_payment
        }
    );
}

#[test]
fn test_cancel_vesting_staked_funds() {
    let mut app = setup_app();

    let amount = Uint128::new(100000000);
    let unchecked_denom = UncheckedDenom::Native(NATIVE_DENOM.to_string());

    // Basic linear vesting schedule
    let start_time = app.block_info().time.plus_seconds(10000).seconds();
    let end_time = app.block_info().time.plus_seconds(30000).seconds();
    let vesting_schedule = Curve::saturating_linear((start_time, amount.into()), (end_time, 0));

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

    // Bob delegates his vesting tokens
    app.execute_contract(
        bob.clone(),
        cw_payroll_addr.clone(),
        &ExecuteMsg::Delegate {
            validator: VALIDATOR.to_string(),
            amount,
        },
        &[],
    )
    .unwrap();

    // Advance the clock
    app.update_block(|block| {
        block.height += 15000;
        block.time = block.time.plus_seconds(15000);
    });

    // Call withdraw rewards before closing
    app.execute_contract(
        bob.clone(),
        cw_payroll_addr.clone(),
        &ExecuteMsg::WithdrawDelegatorReward {
            validator: VALIDATOR.to_string(),
        },
        &[],
    )
    .unwrap();

    // Owner DAO cancels vesting contract
    app.execute_contract(
        owner.clone(),
        cw_payroll_addr.clone(),
        &ExecuteMsg::Cancel {},
        &[],
    )
    .unwrap();

    // Owner DAO can't cancel twice
    app.execute_contract(owner, cw_payroll_addr.clone(), &ExecuteMsg::Cancel {}, &[])
        .unwrap_err();

    // Bob tries to distribute before funds unbond
    app.execute_contract(
        bob.clone(),
        cw_payroll_addr.clone(),
        &ExecuteMsg::Distribute {},
        &[],
    )
    .unwrap_err();

    // Advance the clock
    app.update_block(|block| {
        block.height += 10000;
        block.time = block.time.plus_seconds(100000);
    });

    // Trigger unboding que to return tokens
    app.sudo(SudoMsg::Staking(StakingSudo::ProcessQueue {}))
        .unwrap();

    // Bob tries normal distribute method but can't
    app.execute_contract(
        bob,
        cw_payroll_addr.clone(),
        &ExecuteMsg::Distribute {},
        &[],
    )
    .unwrap();

    // Unvested funds have been returned to contract owner
    assert_eq!(
        get_balance_native(&app, "owner", NATIVE_DENOM),
        Uint128::new(INITIAL_BALANCE) - Uint128::new(25000000)
    );
    // Bob has gets the funds vest up until cancelation, plus staking rewards
    assert_eq!(
        get_balance_native(&app, BOB, NATIVE_DENOM),
        Uint128::new(INITIAL_BALANCE) + Uint128::new(25009512)
    );
    // Contract should have zero funds
    assert_eq!(
        get_balance_native(&app, cw_payroll_addr, NATIVE_DENOM),
        Uint128::zero()
    );
}
