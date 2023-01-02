use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, ReceiveMsg};
use crate::state::{Stream, StreamId, StreamIdsExtensions};
use crate::ContractError;

use cosmwasm_std::testing::mock_info;
use cosmwasm_std::{to_binary, Addr, Empty, Uint128};
use cw20::{Cw20Coin, Cw20ExecuteMsg};
use cw_denom::CheckedDenom;

use cw_multi_test::{App, Contract, ContractWrapper, Executor};
use dao_testing::contracts::cw20_base_contract;

fn cw_payroll_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    );
    Box::new(contract)
}

fn get_stream(app: &App, cw_payroll_addr: Addr, id: u64) -> Stream {
    app.wrap()
        .query_wasm_smart(cw_payroll_addr, &QueryMsg::GetStream { id })
        .unwrap()
}

fn setup_app_and_instantiate_contracts(admin: Option<String>) -> (App, Addr, Addr) {
    let mut app = App::default();

    let cw20_code_id = app.store_code(cw20_base_contract());
    let cw_payroll_code_id = app.store_code(cw_payroll_contract());

    // TODO mint alice and bob native tokens as well

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

    let cw_payroll_addr = app
        .instantiate_contract(
            cw_payroll_code_id,
            Addr::unchecked("ekez"),
            &InstantiateMsg { admin },
            &[],
            "cw-payroll",
            None,
        )
        .unwrap();

    (app, cw20_addr, cw_payroll_addr)
}

#[test]
fn test_execute_distribute() {
    let (mut app, cw20_addr, cw_payroll_addr) = setup_app_and_instantiate_contracts(None);

    let info = mock_info("alice", &[]);

    let recipient = Addr::unchecked("bob").to_string();
    let amount = Uint128::new(1000);

    let denom = CheckedDenom::Cw20(Addr::unchecked("contract0"));
    let claimed = Uint128::zero();
    let start_time = app.block_info().time.plus_seconds(100).seconds();
    let end_time = app.block_info().time.plus_seconds(300).seconds();

    let msg = Cw20ExecuteMsg::Send {
        contract: cw_payroll_addr.to_string(),
        amount,
        msg: to_binary(&ReceiveMsg::CreateStream {
            admin: Some(info.sender.to_string()),
            recipient,
            start_time,
            end_time,
            is_detachable: None,
        })
        .unwrap(),
    };
    app.execute_contract(info.sender.clone(), cw20_addr, &msg, &[])
        .unwrap();

    assert_eq!(
        get_stream(&app, cw_payroll_addr.clone(), 1),
        Stream {
            admin: info.sender.clone(),
            recipient: Addr::unchecked("bob"),
            balance: amount,
            claimed_balance: claimed.clone(),
            denom: denom.clone(),
            start_time,
            end_time,
            title: None,
            description: None,
            paused: false,
            paused_time: None,
            paused_duration: None,
            link_id: None,
            is_detachable: true,
        }
    );

    let bob = Addr::unchecked("bob");

    // Stream has not started
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

    // Stream has started so tokens have vested
    app.execute_contract(
        bob,
        cw_payroll_addr.clone(),
        &ExecuteMsg::Distribute { id: 1 },
        &[],
    )
    .unwrap();

    // Check final balances after distribution
    assert_eq!(
        get_stream(&app, cw_payroll_addr.clone(), 1),
        Stream {
            admin: info.sender,
            recipient: Addr::unchecked("bob"),
            balance: Uint128::new(750),
            claimed_balance: Uint128::new(250),
            denom,
            start_time,
            end_time,
            title: None,
            description: None,
            paused: false,
            paused_time: None,
            paused_duration: None,
            link_id: None,
            is_detachable: true,
        }
    );

    // TODO check bob and alice's balances
}

#[test]
fn test_execute_pause_stream() {
    let (mut app, cw20_addr, cw_payroll_addr) = setup_app_and_instantiate_contracts(None);

    let info = mock_info("alice", &[]);
    let recipient = Addr::unchecked("bob").to_string();
    let amount = Uint128::new(350);
    let start_time = app.block_info().time.plus_seconds(100).seconds();
    let end_time = app.block_info().time.plus_seconds(400).seconds();

    let msg = Cw20ExecuteMsg::Send {
        contract: cw_payroll_addr.to_string(),
        amount,
        msg: to_binary(&ReceiveMsg::CreateStream {
            admin: Some(info.sender.to_string()),
            recipient,
            start_time,
            end_time,
            is_detachable: None,
        })
        .unwrap(),
    };
    app.execute_contract(info.sender.clone(), cw20_addr, &msg, &[])
        .unwrap();

    let stream_id: StreamId = 1;

    app.execute_contract(
        info.sender,
        cw_payroll_addr.clone(),
        &ExecuteMsg::PauseStream { id: stream_id },
        &[],
    )
    .unwrap();

    let denom = CheckedDenom::Cw20(Addr::unchecked("contract0"));
    let saved_stream = get_stream(&app, cw_payroll_addr.clone(), stream_id);
    assert_eq!(
        saved_stream,
        Stream {
            admin: Addr::unchecked("alice"),
            recipient: Addr::unchecked("bob"),
            balance: amount,
            claimed_balance: Uint128::zero(),
            denom,
            start_time,
            end_time,
            title: None,
            description: None,
            paused: true,
            paused_time: saved_stream.paused_time,
            paused_duration: saved_stream.paused_duration,
            link_id: None,
            is_detachable: true,
        }
    );
}

#[test]
fn test_invalid_start_time() {
    let (mut app, cw20_addr, cw_payroll_addr) = setup_app_and_instantiate_contracts(None);

    let info = mock_info("alice", &[]);

    let recipient = Addr::unchecked("bob").to_string();
    let amount = Uint128::new(100);
    let start_time = app.block_info().time.plus_seconds(100).seconds();
    let end_time = app.block_info().time.plus_seconds(20).seconds();

    let msg = Cw20ExecuteMsg::Send {
        contract: cw_payroll_addr.to_string(),
        amount,
        msg: to_binary(&ReceiveMsg::CreateStream {
            admin: Some(info.sender.to_string()),
            recipient,
            start_time,
            end_time,
            is_detachable: None,
        })
        .unwrap(),
    };

    let err: ContractError = app
        .execute_contract(info.sender, cw20_addr, &msg, &[])
        .unwrap_err()
        .downcast()
        .unwrap();

    assert_eq!(err, ContractError::InvalidStartTime {});
}

#[test]
fn test_execute_remove_stream() {
    let (mut app, cw20_addr, cw_payroll_addr) = setup_app_and_instantiate_contracts(None);

    let alice = Addr::unchecked("alice");
    let recipient = Addr::unchecked("bob").to_string();
    let amount = Uint128::new(350);
    let start_time = app.block_info().time.plus_seconds(100).seconds();
    let end_time = app.block_info().time.plus_seconds(400).seconds();

    let msg = Cw20ExecuteMsg::Send {
        contract: cw_payroll_addr.to_string(),
        amount,
        msg: to_binary(&ReceiveMsg::CreateStream {
            admin: Some(alice.to_string()),
            recipient,
            start_time,
            end_time,
            is_detachable: None,
        })
        .unwrap(),
    };
    app.execute_contract(alice, cw20_addr.clone(), &msg, &[])
        .unwrap();

    let stream_id: StreamId = 1;

    // Remove stream and verify not found error returned
    let remove_response = app
        .execute_contract(
            Addr::unchecked("alice"),
            cw_payroll_addr,
            &ExecuteMsg::RemoveStream { id: stream_id },
            &[],
        )
        .unwrap();

    // Make sure refund happened successfully
    assert_eq!(
        remove_response.events[3].attributes[4].value,
        "350".to_string()
    )
}

#[test]
fn test_execute_resume_stream_valid() {
    let (mut app, cw20_addr, cw_payroll_addr) = setup_app_and_instantiate_contracts(None);

    let info = mock_info("alice", &[]);
    let recipient = Addr::unchecked("bob").to_string();
    let amount = Uint128::new(350);
    let start_time = app.block_info().time.plus_seconds(100).seconds();
    let end_time = app.block_info().time.plus_seconds(400).seconds();

    // Create stream 1
    let msg = Cw20ExecuteMsg::Send {
        contract: cw_payroll_addr.to_string(),
        amount,
        msg: to_binary(&ReceiveMsg::CreateStream {
            admin: Some(info.sender.to_string()),
            recipient: recipient.clone(),
            start_time,
            end_time,
            is_detachable: Some(true),
        })
        .unwrap(),
    };
    app.execute_contract(info.sender.clone(), cw20_addr.clone(), &msg, &[])
        .unwrap();

    // Create stream 2
    let msg = Cw20ExecuteMsg::Send {
        contract: cw_payroll_addr.to_string(),
        amount,
        msg: to_binary(&ReceiveMsg::CreateStream {
            admin: Some(info.sender.to_string()),
            recipient,
            start_time,
            end_time,
            is_detachable: Some(true),
        })
        .unwrap(),
    };
    app.execute_contract(info.sender.clone(), cw20_addr.clone(), &msg, &[])
        .unwrap();

    let ids = vec![1, 2];

    app.execute_contract(
        info.sender.clone(),
        cw_payroll_addr.clone(),
        &ExecuteMsg::LinkStream { ids: ids.clone() },
        &[],
    )
    .unwrap();

    app.execute_contract(
        info.sender.clone(),
        cw_payroll_addr.clone(),
        &ExecuteMsg::DetachStream {
            id: *ids.first().unwrap(),
        },
        &[],
    )
    .unwrap();

    let left_stream = get_stream(&app, cw_payroll_addr.clone(), *ids.first().unwrap());
    let right_stream = get_stream(&app, cw_payroll_addr.clone(), *ids.second().unwrap());

    assert!(left_stream.paused);
    assert!(left_stream.paused_time.is_some());
    assert!(left_stream.link_id.is_none());

    assert!(right_stream.paused);
    assert!(right_stream.paused_time.is_some());
    assert!(right_stream.link_id.is_none());

    app.execute_contract(
        info.sender.clone(),
        cw_payroll_addr.clone(),
        &ExecuteMsg::ResumeStream {
            id: *ids.first().unwrap(),
        },
        &[],
    )
    .unwrap();

    let left_stream = get_stream(&app, cw_payroll_addr.clone(), *ids.first().unwrap());
    let right_stream = get_stream(&app, cw_payroll_addr.clone(), *ids.second().unwrap());

    assert!(!left_stream.paused);
    assert!(left_stream.paused_time.is_none());

    assert!(right_stream.paused);
    assert!(right_stream.paused_time.is_some());
}
