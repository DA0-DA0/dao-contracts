use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, ReceiveMsg};
use crate::state::{Stream, StreamId, StreamIdsExtensions};

use super::*;
use cosmwasm_std::testing::{mock_env, mock_info};
use cosmwasm_std::{to_binary, Addr, Empty, Uint128};
use cw20::{Cw20Coin, Cw20ExecuteMsg, Cw20ReceiveMsg};
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

// // TODO rename GetConfig to Config
// fn get_config(app: &App, cw_payroll_addr: Addr) -> Stream {
//     app.wrap()
//         .query_wasm_smart(cw_payroll_addr, &QueryMsg::GetConfig {})
//         .unwrap()
// }

fn setup_app_and_instantiate_contracts(admin: Option<String>) -> (App, Addr, Addr) {
    let mut app = App::default();

    let cw20_code_id = app.store_code(cw20_base_contract());
    let cw_payroll_code_id = app.store_code(cw_payroll_contract());

    let cw20_addr = app
        .instantiate_contract(
            cw20_code_id,
            Addr::unchecked("ekez"),
            &cw20_base::msg::InstantiateMsg {
                name: "cw20 token".to_string(),
                symbol: "cwtwenty".to_string(),
                decimals: 6,
                initial_balances: vec![Cw20Coin {
                    address: "ekez".to_string(),
                    amount: Uint128::new(10),
                }],
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

// #[test]
// fn test_initialization() {
//     let mut deps = mock_dependencies();
//     let msg = InstantiateMsg { admin: None };

//     let info = mock_info("creator", &[]);
//     instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

//     let msg = QueryMsg::GetConfig {};
//     let res = query(deps.as_ref(), mock_env(), msg).unwrap();
//     let config: Config = from_binary(&res).unwrap();

//     assert_eq!(
//         config,
//         Config {
//             admin: Addr::unchecked("creator")
//         }
//     );
// }

#[test]
fn test_execute_distribute() {
    let (mut app, cw20_addr, cw_payroll_addr) = setup_app_and_instantiate_contracts(None);

    let info = mock_info("alice", &[]);

    let recipient = Addr::unchecked("bob").to_string();
    let amount = Uint128::new(1000);

    let denom = CheckedDenom::Cw20(Addr::unchecked("cw20"));
    let claimed = Uint128::zero();
    let env = mock_env();
    let start_time = env.block.time.plus_seconds(100).seconds();
    let end_time = env.block.time.plus_seconds(300).seconds();

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

    // Stream has not started
    let mut info = mock_info("owner", &[]);
    info.sender = Addr::unchecked("bob");
    let msg = ExecuteMsg::Distribute { id: 1 };
    let err: ContractError = app
        .execute_contract(info.sender.clone(), cw_payroll_addr.clone(), &msg, &[])
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(err, ContractError::NoFundsToClaim { claimed });

    // Stream has started so tokens have vested
    let msg = ExecuteMsg::Distribute { id: 1 };
    let mut info = mock_info("owner", &[]);
    let mut env = mock_env();
    let sender = Addr::unchecked("bob");
    info.sender = sender;
    env.block.time = env.block.time.plus_seconds(150);
    app.execute_contract(info.sender.clone(), cw_payroll_addr.clone(), &msg, &[])
        .unwrap();

    //// TODO check event messages
    // let msg = res.messages[0].clone().msg;
    // assert_eq!(
    //     msg,
    //     CosmosMsg::Wasm(WasmMsg::Execute {
    //         contract_addr: String::from("cw20"),
    //         msg: to_binary(&Cw20ExecuteMsg::Transfer {
    //             recipient: String::from("bob"),
    //             amount: Uint128::new(250)
    //         })
    //         .unwrap(),
    //         funds: vec![]
    //     })
    // );

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
}

#[test]
fn test_create_stream_with_refund() {
    let (mut app, cw20_addr, cw_payroll_addr) = setup_app_and_instantiate_contracts(None);

    let info = mock_info("alice", &[]);
    let recipient = Addr::unchecked("bob").to_string();
    let amount = Uint128::new(350);
    let env = mock_env();
    let start_time = env.block.time.plus_seconds(100).seconds();
    let end_time = env.block.time.plus_seconds(400).seconds();

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

    // Make sure remaining funds were refunded if duration didn't divide evenly into amount
    // let refund_msg = res.messages[0].clone().msg;
    // assert_eq!(
    //     refund_msg,
    //     CosmosMsg::Wasm(WasmMsg::Execute {
    //         contract_addr: String::from("cw20"),
    //         msg: to_binary(&Cw20ExecuteMsg::Transfer {
    //             recipient: sender,
    //             amount: Uint128::new(50)
    //         })
    //         .unwrap(),
    //         funds: vec![]
    //     })
    // );

    let balance = Uint128::new(350);
    let denom = CheckedDenom::Cw20(Addr::unchecked("cw20"));

    assert_eq!(
        get_stream(&app, cw_payroll_addr.clone(), 1),
        Stream {
            admin: info.sender,
            recipient: Addr::unchecked("bob"),
            balance, // original amount - refund
            claimed_balance: Uint128::zero(),
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
}
#[test]
fn test_execute_pause_stream() {
    let (mut app, cw20_addr, cw_payroll_addr) = setup_app_and_instantiate_contracts(None);

    let sender = Addr::unchecked("alice").to_string();
    let info = mock_info(&sender, &[]);

    let recipient = Addr::unchecked("bob").to_string();
    let amount = Uint128::new(350);
    let env = mock_env();
    let start_time = env.block.time.plus_seconds(100).seconds();
    let end_time = env.block.time.plus_seconds(400).seconds();

    let msg = Cw20ExecuteMsg::Send {
        contract: cw_payroll_addr.to_string(),
        amount,
        msg: to_binary(&ReceiveMsg::CreateStream {
            admin: Some(sender),
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
        cw_payroll_addr.clone(),
        info.sender,
        &ExecuteMsg::PauseStream { id: stream_id },
        &[],
    )
    .unwrap();

    let denom = CheckedDenom::Cw20(Addr::unchecked("cw20"));
    let saved_stream = get_stream(&app, cw_payroll_addr.clone(), stream_id);
    assert_eq!(
        saved_stream,
        Stream {
            admin: Addr::unchecked("alice"),
            recipient: Addr::unchecked("bob"),
            balance: amount, // original amount - refund
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
    let start_time = mock_env().block.time.plus_seconds(100).seconds();
    let end_time = mock_env().block.time.plus_seconds(20).seconds();

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
        .execute_contract(cw20_addr, info.sender, &msg, &[])
        .unwrap_err()
        .downcast()
        .unwrap();

    assert_eq!(err, ContractError::InvalidStartTime {});
}

#[test]
fn invalid_cw20_addr() {
    let (mut app, cw20_addr, cw_payroll_addr) = setup_app_and_instantiate_contracts(None);

    let mut info = mock_info("alice", &[]);

    let recipient = Addr::unchecked("bob").to_string();
    let amount = Uint128::new(100);
    let start_time = mock_env().block.time.plus_seconds(100).seconds();
    let end_time = mock_env().block.time.plus_seconds(200).seconds();

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

    // TODO clean up
    info.sender = Addr::unchecked("wrongCw20");

    let err: ContractError = app
        .execute_contract(cw20_addr, info.sender, &msg, &[])
        .unwrap_err()
        .downcast()
        .unwrap();

    assert_eq!(
        err,
        ContractError::Std(cosmwasm_std::StdError::GenericErr {
            msg: "Invalid input: address not normalized".to_string()
        })
    );
}

//// Do we want this?
// #[test]
// fn test_invalid_deposit_amount() {
//     let mut deps = mock_dependencies();

//     let msg = InstantiateMsg { admin: None };
//     let mut info = mock_info("alice", &[]);
//     instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

//     let sender = Addr::unchecked("alice").to_string();
//     let recipient = Addr::unchecked("bob").to_string();
//     let amount = Uint128::new(3);
//     let start_time = mock_env().block.time.plus_seconds(100).seconds();
//     let end_time = mock_env().block.time.plus_seconds(200).seconds();

//     let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
//         sender,
//         amount,
//         msg: to_binary(&ReceiveMsg::CreateStream {
//             admin: None,
//             recipient,
//             start_time,
//             end_time,
//             is_detachable: None,
//         })
//         .unwrap(),
//     });
//     info.sender = Addr::unchecked("cw20");
//     let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
//     assert_eq!(err, ContractError::AmountLessThanDuration {});
// }

#[test]
fn test_execute_remove_stream() {
    let (mut app, cw20_addr, cw_payroll_addr) = setup_app_and_instantiate_contracts(None);

    let alice = Addr::unchecked("alice");
    let recipient = Addr::unchecked("bob").to_string();
    let amount = Uint128::new(350);
    let env = mock_env();
    let start_time = env.block.time.plus_seconds(100).seconds();
    let end_time = env.block.time.plus_seconds(400).seconds();

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

    // TODO cleanup addresses
    // Remove stream and verify not found error returned
    let remove_response = app
        .execute_contract(
            Addr::unchecked("alice"),
            cw20_addr,
            &ExecuteMsg::RemoveStream { id: stream_id },
            &[],
        )
        .unwrap();

    // TODO test query error message
    // let error = get_stream(&app, cw_payroll_addr, stream_id);
    // assert_eq!(
    //     error,
    //     StdError::NotFound {
    //         kind: "cw_payroll::state::Stream".to_string()
    //     }
    // );

    // TODO check
    // let refund_msg = remove_response.messages[0].clone().msg;
    // assert_eq!(
    //     refund_msg,
    //     CosmosMsg::Wasm(WasmMsg::Execute {
    //         contract_addr: sender.clone(),
    //         msg: to_binary(&Cw20ExecuteMsg::Transfer {
    //             recipient: sender,
    //             amount
    //         })
    //         .unwrap(),
    //         funds: vec![]
    //     })
    // );
}
#[test]
fn test_execute_link_stream_invalid() {
    let (mut app, cw20_addr, cw_payroll_addr) = setup_app_and_instantiate_contracts(None);

    let info = mock_info("alice", &[]);

    let recipient = Addr::unchecked("bob").to_string();
    let amount = Uint128::new(350);
    let env = mock_env();
    let start_time = env.block.time.plus_seconds(100).seconds();
    let end_time = env.block.time.plus_seconds(400).seconds();

    //Create stream 1
    let msg = Cw20ExecuteMsg::Send {
        contract: cw_payroll_addr.to_string(),
        amount,
        msg: to_binary(&ReceiveMsg::CreateStream {
            admin: Some(info.sender.to_string()),
            recipient: recipient.clone(),
            start_time,
            end_time,
            is_detachable: None,
        })
        .unwrap(),
    };

    // TODO this is the wrong type of message above
    app.execute_contract(info.sender.clone(), cw20_addr.clone(), &msg, &[])
        .unwrap();

    let ids = vec![1, 2];

    //Link stream and verify error returned
    let error: ContractError = app
        .execute_contract(
            info.sender.clone(),
            cw_payroll_addr.clone(),
            &ExecuteMsg::LinkStream { ids: ids.clone() },
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();

    assert_eq!(
        error,
        ContractError::StreamNotFound {
            stream_id: *ids.second().unwrap()
        }
    );

    let ids = vec![1, 1];

    //Link stream and verify error returned
    app.execute_contract(
        info.sender.clone(),
        cw_payroll_addr.clone(),
        &ExecuteMsg::LinkStream { ids: ids.clone() },
        &[],
    )
    .unwrap_err();
    // assert_eq!(error, ContractError::StreamsShouldNotBeEqual {});

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
    app.execute_contract(info.sender, cw20_addr, &msg, &[])
        .unwrap();

    let sender = Addr::unchecked("bob").to_string();
    let ids = vec![1, 2];

    let unauthorized_info = mock_info(&sender, &[]);
    let error: ContractError = app
        .execute_contract(
            unauthorized_info.sender,
            cw_payroll_addr.clone(),
            &ExecuteMsg::LinkStream { ids: ids.clone() },
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(error, ContractError::Unauthorized {});
}

#[test]
fn test_execute_link_stream_valid() {
    let (mut app, cw20_addr, cw_payroll_addr) = setup_app_and_instantiate_contracts(None);

    let info = mock_info("alice", &[]);

    let recipient = Addr::unchecked("bob").to_string();
    let amount = Uint128::new(350);
    let env = mock_env();
    let start_time = env.block.time.plus_seconds(100).seconds();
    let end_time = env.block.time.plus_seconds(400).seconds();

    //Create stream 1
    let msg = Cw20ExecuteMsg::Send {
        contract: cw_payroll_addr.to_string(),
        amount,
        msg: to_binary(&ReceiveMsg::CreateStream {
            admin: Some(info.sender.to_string()),
            recipient: recipient.clone(),
            start_time,
            end_time,
            is_detachable: None,
        })
        .unwrap(),
    };
    app.execute_contract(info.sender.clone(), cw20_addr.clone(), &msg, &[])
        .unwrap();

    //Create stream 2
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

    // TODO this is the wrong type of message above
    app.execute_contract(info.sender.clone(), cw20_addr, &msg, &[])
        .unwrap();

    let ids = vec![1, 2];
    let response = app
        .execute_contract(
            info.sender.clone(),
            cw_payroll_addr.clone(),
            &ExecuteMsg::LinkStream { ids: ids.clone() },
            &[],
        )
        .unwrap();

    let left_stream = get_stream(&app, cw_payroll_addr.clone(), *ids.first().unwrap());
    let right_stream = get_stream(&app, cw_payroll_addr.clone(), *ids.second().unwrap());
    assert_eq!(left_stream.link_id, Some(*ids.second().unwrap()));
    assert_eq!(right_stream.link_id, Some(*ids.first().unwrap()));

    // TODO maybe check?
    // assert!(response
    //     .attributes
    //     .iter()
    //     .any(|f| { f.key == "left_stream_id" }));
    // assert!(response
    //     .attributes
    //     .iter()
    //     .any(|f| { f.key == "right_stream_id" }));
}

#[test]
fn test_execute_detach_stream_valid() {
    let (mut app, cw20_addr, cw_payroll_addr) = setup_app_and_instantiate_contracts(None);

    let info = mock_info("sender", &[]);
    let recipient = Addr::unchecked("bob").to_string();
    let amount = Uint128::new(350);
    let env = mock_env();
    let start_time = env.block.time.plus_seconds(100).seconds();
    let end_time = env.block.time.plus_seconds(400).seconds();

    // Create stream 1
    let msg = Cw20ExecuteMsg::Send {
        contract: cw_payroll_addr.to_string(),
        amount,
        msg: to_binary(&ReceiveMsg::CreateStream {
            admin: Some(info.sender.to_string()),
            recipient: recipient.clone(),
            start_time,
            end_time,
            is_detachable: None,
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
            is_detachable: None,
        })
        .unwrap(),
    };
    app.execute_contract(info.sender.clone(), cw20_addr, &msg, &[])
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
}

#[test]
fn test_execute_detach_stream_invalid() {
    let (mut app, cw20_addr, cw_payroll_addr) = setup_app_and_instantiate_contracts(None);

    let info = mock_info("alice", &[]);

    let recipient = Addr::unchecked("bob").to_string();
    let amount = Uint128::new(350);
    let env = mock_env();
    let start_time = env.block.time.plus_seconds(100).seconds();
    let end_time = env.block.time.plus_seconds(400).seconds();

    // Create stream 1
    let msg = Cw20ExecuteMsg::Send {
        contract: cw_payroll_addr.to_string(),
        amount,
        msg: to_binary(&ReceiveMsg::CreateStream {
            admin: Some(info.sender.to_string()),
            recipient: recipient.clone(),
            start_time,
            end_time,
            is_detachable: Some(false),
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
            recipient: recipient.clone(),
            start_time,
            end_time,
            is_detachable: Some(false),
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

    let ids = vec![11, 22];
    let error: ContractError = app
        .execute_contract(
            info.sender.clone(),
            cw_payroll_addr.clone(),
            &ExecuteMsg::DetachStream {
                id: *ids.first().unwrap(),
            },
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(
        error,
        ContractError::StreamNotFound {
            stream_id: *ids.first().unwrap()
        }
    );

    let ids = vec![1, 22];
    let error: ContractError = app
        .execute_contract(
            info.sender.clone(),
            cw_payroll_addr.clone(),
            &ExecuteMsg::DetachStream {
                id: *ids.second().unwrap(),
            },
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(
        error,
        ContractError::StreamNotFound {
            stream_id: *ids.second().unwrap()
        }
    );

    let ids = vec![1, 2];
    let error: ContractError = app
        .execute_contract(
            info.sender.clone(),
            cw_payroll_addr.clone(),
            &ExecuteMsg::DetachStream {
                id: *ids.second().unwrap(),
            },
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(error, ContractError::StreamNotDetachable {});

    let unauthorized_info = mock_info(&recipient, &[]);

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

    let ids = vec![3, 4];
    app.execute_contract(
        cw_payroll_addr.clone(),
        info.sender,
        &ExecuteMsg::LinkStream { ids: ids.clone() },
        &[],
    )
    .unwrap();

    let error: ContractError = app
        .execute_contract(
            cw_payroll_addr.clone(),
            unauthorized_info.sender,
            &ExecuteMsg::DetachStream {
                id: *ids.first().unwrap(),
            },
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();

    assert_eq!(error, ContractError::Unauthorized {});
}

#[test]
fn test_execute_resume_stream_valid() {
    let (mut app, cw20_addr, cw_payroll_addr) = setup_app_and_instantiate_contracts(None);

    let sender = Addr::unchecked("alice").to_string();

    let info = mock_info(&sender, &[]);

    let recipient = Addr::unchecked("bob").to_string();
    let amount = Uint128::new(350);
    let env = mock_env();
    let start_time = env.block.time.plus_seconds(100).seconds();
    let end_time = env.block.time.plus_seconds(400).seconds();

    //Create stream 1
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: sender.clone(),
        amount,
        msg: to_binary(&ReceiveMsg::CreateStream {
            admin: Some(sender.clone()),
            recipient: recipient.clone(),
            start_time,
            end_time,
            is_detachable: None,
        })
        .unwrap(),
    });
    // TODO this is the wrong type of message above
    app.execute_contract(info.sender.clone(), cw20_addr.clone(), &msg, &[])
        .unwrap();

    //Create stream 2
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: sender.clone(),
        amount,
        msg: to_binary(&ReceiveMsg::CreateStream {
            admin: Some(sender),
            recipient,
            start_time,
            end_time,
            is_detachable: None,
        })
        .unwrap(),
    });
    // TODO this is the wrong type of message above
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

#[test]
fn test_execute_resume_stream_invalid() {
    let (mut app, cw20_addr, cw_payroll_addr) = setup_app_and_instantiate_contracts(None);

    let sender = Addr::unchecked("alice").to_string();

    let info = mock_info(&sender, &[]);

    let recipient = Addr::unchecked("bob").to_string();
    let amount = Uint128::new(350);
    let env = mock_env();
    let start_time = env.block.time.plus_seconds(100).seconds();
    let end_time = env.block.time.plus_seconds(400).seconds();

    //Create stream 1
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: sender.clone(),
        amount,
        msg: to_binary(&ReceiveMsg::CreateStream {
            admin: Some(sender.clone()),
            recipient: recipient.clone(),
            start_time,
            end_time,
            is_detachable: Some(true),
        })
        .unwrap(),
    });
    // TODO this is the wrong type of message above
    app.execute_contract(info.sender.clone(), cw20_addr.clone(), &msg, &[])
        .unwrap();

    //Create stream 2
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: sender.clone(),
        amount,
        msg: to_binary(&ReceiveMsg::CreateStream {
            admin: Some(sender),
            recipient: recipient.clone(),
            start_time,
            end_time,
            is_detachable: Some(false),
        })
        .unwrap(),
    });
    // TODO this is the wrong type of message above
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

    let error: ContractError = app
        .execute_contract(
            cw_payroll_addr.clone(),
            info.sender.clone(),
            &ExecuteMsg::ResumeStream {
                id: *ids.second().unwrap(),
            },
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(error, ContractError::StreamNotPaused {});

    let unauthorized_info = mock_info(&recipient, &[]);
    let error: ContractError = app
        .execute_contract(
            unauthorized_info.sender,
            cw_payroll_addr.clone(),
            &ExecuteMsg::DetachStream {
                id: *ids.first().unwrap(),
            },
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(error, ContractError::StreamNotDetachable {});
}
