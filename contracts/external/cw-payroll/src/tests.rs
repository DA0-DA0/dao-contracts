use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, ReceiveMsg, VestingParams};
use crate::state::VestingPayment;
use crate::ContractError;

use cosmwasm_std::testing::mock_info;
use cosmwasm_std::{coins, to_binary, Addr, Empty, Uint128};
use cw20::{Cw20Coin, Cw20ExecuteMsg};
use cw_denom::CheckedDenom;

use cw_multi_test::{App, BankSudo, Contract, ContractWrapper, Executor, SudoMsg};
use dao_testing::contracts::cw20_base_contract;
use wynd_utils::Curve;

const NATIVE_DENOM: &str = "ujuno";
const ALICE: &str = "alice";
const BOB: &str = "bob";

fn cw_payroll_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    );
    Box::new(contract)
}

fn get_stream(app: &App, cw_payroll_addr: Addr, id: u64) -> VestingPayment {
    app.wrap()
        .query_wasm_smart(cw_payroll_addr, &QueryMsg::GetVestingPayment { id })
        .unwrap()
}

fn setup_app_and_instantiate_contracts(admin: Option<String>) -> (App, Addr, Addr) {
    let mut app = App::default();

    let cw20_code_id = app.store_code(cw20_base_contract());
    let cw_payroll_code_id = app.store_code(cw_payroll_contract());

    // Mint Alice and Bob native tokens
    app.sudo(SudoMsg::Bank({
        BankSudo::Mint {
            to_address: ALICE.to_string(),
            amount: coins(100000, NATIVE_DENOM),
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
                        address: "alice".to_string(),
                        amount: Uint128::new(10000),
                    },
                    Cw20Coin {
                        address: "bob".to_string(),
                        amount: Uint128::new(1000),
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
            &InstantiateMsg {
                admin,
                create_new_vesting_schedule_params: None,
            },
            &[],
            "cw-payroll",
            None,
        )
        .unwrap();

    (app, cw20_addr, cw_payroll_addr)
}

#[test]
fn test_happy_path() {
    let (mut app, cw20_addr, cw_payroll_addr) = setup_app_and_instantiate_contracts(None);

    let info = mock_info("alice", &[]);

    let recipient = Addr::unchecked("bob").to_string();
    let amount = Uint128::new(1000);

    let denom = CheckedDenom::Cw20(Addr::unchecked("contract0"));
    let claimed = Uint128::zero();
    let start_time = app.block_info().time.plus_seconds(100).seconds();
    let end_time = app.block_info().time.plus_seconds(300).seconds();
    let vesting_schedule = Curve::saturating_linear((start_time, amount.into()), (end_time, 0));

    let msg = Cw20ExecuteMsg::Send {
        contract: cw_payroll_addr.to_string(),
        amount,
        msg: to_binary(&ReceiveMsg::Create(VestingParams {
            recipient,
            amount,
            denom: denom.clone(),
            vesting_schedule: vesting_schedule.clone(),
            title: None,
            description: None,
        }))
        .unwrap(),
    };
    app.execute_contract(info.sender.clone(), cw20_addr, &msg, &[])
        .unwrap();

    assert_eq!(
        get_stream(&app, cw_payroll_addr.clone(), 1),
        VestingPayment {
            recipient: Addr::unchecked("bob"),
            amount: amount,
            claimed_amount: claimed.clone(),
            denom: denom.clone(),
            vesting_schedule: vesting_schedule.clone(),
            title: None,
            description: None,
            paused: false,
        }
    );

    let bob = Addr::unchecked("bob");

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
    assert_eq!(err, ContractError::NoFundsToClaim { claimed });

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
    );

    // Check final amounts after distribution
    assert_eq!(
        get_stream(&app, cw_payroll_addr.clone(), 1),
        VestingPayment {
            recipient: Addr::unchecked("bob"),
            amount: Uint128::new(750),
            claimed_amount: Uint128::new(250),
            denom,
            vesting_schedule,
            title: None,
            description: None,
            paused: false,
        }
    );

    // TODO check bob and alice's balances
}
