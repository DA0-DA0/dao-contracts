use cosmwasm_std::{coins, to_binary, Addr, Empty, Uint128};
use cw20::{Cw20Coin, Cw20ExecuteMsg};
use cw_denom::{CheckedDenom, UncheckedDenom};
use cw_multi_test::{App, BankSudo, Contract, ContractWrapper, Executor, SudoMsg};
use dao_testing::contracts::cw20_base_contract;
use wynd_utils::Curve;

use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, ReceiveMsg};
use crate::state::{UncheckedVestingParams, VestingPayment, VestingPaymentStatus};
use crate::ContractError;

const NATIVE_DENOM: &str = "ujuno";
const ALICE: &str = "alice";
const BOB: &str = "bob";
const INITIAL_BALANCE: u128 = 10000;

fn cw_payroll_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    );
    Box::new(contract)
}

fn get_vesting_payment(app: &App, cw_payroll_addr: Addr, id: u64) -> VestingPayment {
    app.wrap()
        .query_wasm_smart(cw_payroll_addr, &QueryMsg::GetVestingPayment { id })
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

fn setup_app_and_instantiate_contracts(owner: Option<String>) -> (App, Addr, Addr) {
    let mut app = App::default();

    let cw20_code_id = app.store_code(cw20_base_contract());
    let cw_payroll_code_id = app.store_code(cw_payroll_contract());

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

    // Instantiate cw20 contract with balances for Alice and Bob
    let cw20_addr = app
        .instantiate_contract(
            cw20_code_id,
            Addr::unchecked("ekez"),
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
                ],
                mint: None,
                marketing: None,
            },
            &[],
            "cw20-base",
            None,
        )
        .unwrap();

    // Instantiate cw-payroll contract
    let cw_payroll_addr = app
        .instantiate_contract(
            cw_payroll_code_id,
            Addr::unchecked("ekez"),
            &InstantiateMsg { owner },
            &[],
            "cw-payroll",
            None,
        )
        .unwrap();

    (app, cw20_addr, cw_payroll_addr)
}

struct TestCase {
    alice: Addr,
    bob: Addr,
    vesting_payment: VestingPayment,
}

fn setup_test_case(
    app: &mut App,
    cw_payroll_addr: Addr,
    vesting_schedule: Curve,
    amount: Uint128,
    denom: UncheckedDenom,
) -> TestCase {
    let vesting_payment = match denom {
        UncheckedDenom::Cw20(ref cw20_addr) => {
            let msg = Cw20ExecuteMsg::Send {
                contract: cw_payroll_addr.to_string(),
                amount,
                msg: to_binary(&ReceiveMsg::Create(UncheckedVestingParams {
                    recipient: BOB.to_string(),
                    amount,
                    denom: denom.clone(),
                    vesting_schedule: vesting_schedule.clone(),
                    title: None,
                    description: None,
                }))
                .unwrap(),
            };
            app.execute_contract(
                Addr::unchecked(ALICE),
                Addr::unchecked(cw20_addr.clone()),
                &msg,
                &[],
            )
            .unwrap();

            let vp = get_vesting_payment(&app, cw_payroll_addr.clone(), 1);

            // Check Vesting Payment was created correctly
            assert_eq!(
                vp,
                VestingPayment {
                    id: 1,
                    recipient: Addr::unchecked(BOB),
                    amount: amount,
                    claimed_amount: Uint128::zero(),
                    denom: CheckedDenom::Cw20(Addr::unchecked(cw20_addr)),
                    vesting_schedule: vesting_schedule.clone(),
                    title: None,
                    description: None,
                    status: VestingPaymentStatus::Active,
                }
            );

            vp
        }
        UncheckedDenom::Native(_) => {
            let msg = ExecuteMsg::Create(UncheckedVestingParams {
                recipient: BOB.to_string(),
                amount,
                denom,
                vesting_schedule: vesting_schedule.clone(),
                title: None,
                description: None,
            });
            app.execute_contract(
                Addr::unchecked(ALICE),
                cw_payroll_addr.clone(),
                &msg,
                &coins(amount.into(), NATIVE_DENOM.to_string()),
            )
            .unwrap();

            let vp = get_vesting_payment(&app, cw_payroll_addr.clone(), 1);

            // Check Vesting Payment was created correctly
            assert_eq!(
                vp,
                VestingPayment {
                    id: 1,
                    recipient: Addr::unchecked(BOB),
                    amount,
                    claimed_amount: Uint128::zero(),
                    denom: CheckedDenom::Native(NATIVE_DENOM.to_string()),
                    vesting_schedule: vesting_schedule.clone(),
                    title: None,
                    description: None,
                    status: VestingPaymentStatus::Active,
                }
            );

            vp
        }
    };

    TestCase {
        alice: Addr::unchecked(ALICE),
        bob: Addr::unchecked(BOB),
        vesting_payment,
    }
}

#[test]
fn test_happy_cw20_path() {
    let (mut app, cw20_addr, cw_payroll_addr) = setup_app_and_instantiate_contracts(None);

    let amount = Uint128::new(1000);
    let unchecked_denom = UncheckedDenom::Cw20(cw20_addr.to_string());

    // Basic linear vesting schedule
    let start_time = app.block_info().time.plus_seconds(100).seconds();
    let end_time = app.block_info().time.plus_seconds(300).seconds();
    let vesting_schedule = Curve::saturating_linear((start_time, amount.into()), (end_time, 0));

    let TestCase {
        bob,
        vesting_payment,
        ..
    } = setup_test_case(
        &mut app,
        cw_payroll_addr.clone(),
        vesting_schedule.clone(),
        amount,
        unchecked_denom,
    );

    // VestingPayment has not started
    let err: ContractError = app
        .execute_contract(
            bob.clone(),
            cw_payroll_addr.clone(),
            &ExecuteMsg::Distribute { id: 1 },
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(
        err,
        ContractError::NoFundsToClaim {
            claimed: Uint128::zero()
        }
    );

    // Advance the clock
    app.update_block(|block| {
        block.time = block.time.plus_seconds(150);
    });

    // VestingPayment has started so tokens have vested
    app.execute_contract(
        bob,
        cw_payroll_addr.clone(),
        &ExecuteMsg::Distribute { id: 1 },
        &[],
    )
    .unwrap();

    // Check final amounts after distribution
    assert_eq!(
        get_vesting_payment(&app, cw_payroll_addr.clone(), 1),
        VestingPayment {
            id: 1,
            recipient: Addr::unchecked(BOB),
            amount: Uint128::new(750),
            claimed_amount: Uint128::new(250),
            ..vesting_payment
        }
    );

    // Alice has funded the contract and down 1000
    assert_eq!(
        get_balance_cw20(&app, cw20_addr.clone(), ALICE),
        Uint128::new(INITIAL_BALANCE) - amount
    );
    // Bob has claimed vested funds and is up 250
    assert_eq!(get_balance_cw20(&app, cw20_addr, BOB), Uint128::new(10250));

    // TODO finish up vesting period, check status
}

#[test]
fn test_happy_native_path() {
    let (mut app, _, cw_payroll_addr) = setup_app_and_instantiate_contracts(None);

    let amount = Uint128::new(1000);
    let unchecked_denom = UncheckedDenom::Native(NATIVE_DENOM.to_string());

    // Basic linear vesting schedule
    let start_time = app.block_info().time.plus_seconds(100).seconds();
    let end_time = app.block_info().time.plus_seconds(300).seconds();
    let vesting_schedule = Curve::saturating_linear((start_time, amount.into()), (end_time, 0));

    let TestCase {
        bob,
        vesting_payment,
        ..
    } = setup_test_case(
        &mut app,
        cw_payroll_addr.clone(),
        vesting_schedule.clone(),
        amount,
        unchecked_denom,
    );

    // VestingPayment has not started
    let err: ContractError = app
        .execute_contract(
            bob.clone(),
            cw_payroll_addr.clone(),
            &ExecuteMsg::Distribute { id: 1 },
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(
        err,
        ContractError::NoFundsToClaim {
            claimed: Uint128::zero()
        }
    );

    // Advance the clock
    app.update_block(|block| {
        block.time = block.time.plus_seconds(150);
    });

    // VestingPayment has started so tokens have vested
    app.execute_contract(
        bob,
        cw_payroll_addr.clone(),
        &ExecuteMsg::Distribute { id: 1 },
        &[],
    )
    .unwrap();

    // Check final amounts after distribution
    assert_eq!(
        get_vesting_payment(&app, cw_payroll_addr.clone(), 1),
        VestingPayment {
            id: 1,
            recipient: Addr::unchecked(BOB),
            amount: Uint128::new(750),
            claimed_amount: Uint128::new(250),
            ..vesting_payment
        }
    );

    // Alice has funded the contract and down 1000
    assert_eq!(
        get_balance_native(&app, ALICE, NATIVE_DENOM),
        Uint128::new(INITIAL_BALANCE) - amount
    );
    // Bob has claimed vested funds and is up 250
    assert_eq!(
        get_balance_native(&app, BOB, NATIVE_DENOM),
        Uint128::new(10250)
    );

    // TODO finish up vesting period, check status
}

#[test]
fn test_cancel_vesting() {
    // TODO only admin can cancel
    // TODO unvested funds are returned to owner
}
