use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, ReceiveMsg};
use crate::state::{Config, Stream, StreamId, StreamIdsExtensions};

use super::*;
use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::{
    from_binary, to_binary, Addr, CosmosMsg, Deps, StdError, StdResult, Uint128, WasmMsg,
};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};
use cw_denom::CheckedDenom;

// TODO use multitest
use dao_testing::contracts::cw20_base_contract;

fn get_stream(deps: Deps, id: u64) -> StdResult<Stream> {
    let msg = QueryMsg::GetStream { id };
    let res = query(deps, mock_env(), msg)?;
    from_binary::<Stream>(&res)
}

// TODO get config query

#[test]
fn test_initialization() {
    let mut deps = mock_dependencies();
    let msg = InstantiateMsg { admin: None };

    let info = mock_info("creator", &[]);
    instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = QueryMsg::GetConfig {};
    let res = query(deps.as_ref(), mock_env(), msg).unwrap();
    let config: Config = from_binary(&res).unwrap();

    assert_eq!(
        config,
        Config {
            admin: Addr::unchecked("creator")
        }
    );
}

#[test]
fn test_execute_distribute() {
    let mut deps = mock_dependencies();
    let msg = InstantiateMsg { admin: None };
    let info = mock_info("cw20", &[]);
    instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let admin_addr = Addr::unchecked("cw20");
    let sender = admin_addr.to_string();

    let recipient = Addr::unchecked("bob").to_string();
    let amount = Uint128::new(1000);

    let denom = CheckedDenom::Cw20(Addr::unchecked("cw20"));
    let claimed = Uint128::zero();
    let env = mock_env();
    let start_time = env.block.time.plus_seconds(100).seconds();
    let end_time = env.block.time.plus_seconds(300).seconds();
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
    execute(deps.as_mut(), env.clone(), info, msg).unwrap();

    assert_eq!(
        get_stream(deps.as_ref(), 1).unwrap(),
        Stream {
            admin: admin_addr.clone(),
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
    let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(err, ContractError::NoFundsToClaim { claimed });

    // Stream has started so tokens have vested
    let msg = ExecuteMsg::Distribute { id: 1 };
    let mut info = mock_info("owner", &[]);
    let mut env = mock_env();
    let sender = Addr::unchecked("bob");
    info.sender = sender;
    env.block.time = env.block.time.plus_seconds(150);
    let res = execute(deps.as_mut(), env, info, msg).unwrap();
    let msg = res.messages[0].clone().msg;

    assert_eq!(
        msg,
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: String::from("cw20"),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: String::from("bob"),
                amount: Uint128::new(250)
            })
            .unwrap(),
            funds: vec![]
        })
    );
    assert_eq!(
        get_stream(deps.as_ref(), 1).unwrap(),
        Stream {
            admin: admin_addr,
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

    //Check final balances after distribution
}

#[test]
fn test_create_stream_with_refund() {
    let mut deps = mock_dependencies();
    let msg = InstantiateMsg { admin: None };
    let info = mock_info("cw20", &[]);
    instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let recipient = Addr::unchecked("bob");
    let sender = info.sender.clone().into_string();
    let sender_addr = info.sender.clone();

    let amount = Uint128::new(350);
    let env = mock_env();
    let start_time = env.block.time.plus_seconds(100).seconds();
    let end_time = env.block.time.plus_seconds(400).seconds();

    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: sender.clone(),
        amount,
        msg: to_binary(&ReceiveMsg::CreateStream {
            admin: None,
            recipient: recipient.into_string(),
            start_time,
            end_time,
            is_detachable: None,
        })
        .unwrap(),
    });

    // Make sure remaining funds were refunded if duration didn't divide evenly into amount
    let res = execute(deps.as_mut(), env, info, msg).unwrap();
    let refund_msg = res.messages[0].clone().msg;
    assert_eq!(
        refund_msg,
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: String::from("cw20"),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: sender,
                amount: Uint128::new(50)
            })
            .unwrap(),
            funds: vec![]
        })
    );

    let balance = Uint128::new(350);
    let denom = CheckedDenom::Cw20(Addr::unchecked("cw20"));

    assert_eq!(
        get_stream(deps.as_ref(), 1).unwrap(),
        Stream {
            admin: sender_addr,
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
    let mut deps = mock_dependencies();
    let msg = InstantiateMsg { admin: None };
    let sender = Addr::unchecked("alice").to_string();

    let sender_addr = Addr::unchecked("alice");
    let info = mock_info(&sender, &[]);
    instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let recipient = Addr::unchecked("bob").to_string();
    let amount = Uint128::new(350);
    let env = mock_env();
    let start_time = env.block.time.plus_seconds(100).seconds();
    let end_time = env.block.time.plus_seconds(400).seconds();

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
    execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let stream_id: StreamId = 1;

    let _ = execute(
        deps.as_mut(),
        mock_env(),
        info,
        ExecuteMsg::PauseStream { id: stream_id },
    )
    .unwrap();

    let denom = CheckedDenom::Cw20(Addr::unchecked("cw20"));
    let saved_stream = get_stream(deps.as_ref(), stream_id).unwrap();
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
    let mut deps = mock_dependencies();

    let msg = InstantiateMsg { admin: None };
    let mut info = mock_info("alice", &[]);
    instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let sender = Addr::unchecked("alice").to_string();
    let recipient = Addr::unchecked("bob").to_string();
    let amount = Uint128::new(100);
    let start_time = mock_env().block.time.plus_seconds(100).seconds();
    let end_time = mock_env().block.time.plus_seconds(20).seconds();

    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender,
        amount,
        msg: to_binary(&ReceiveMsg::CreateStream {
            admin: None,
            recipient,
            start_time,
            end_time,
            is_detachable: None,
        })
        .unwrap(),
    });
    info.sender = Addr::unchecked("cw20");
    let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(err, ContractError::InvalidStartTime {});
}

#[test]
fn invalid_cw20_addr() {
    let mut deps = mock_dependencies();

    let msg = InstantiateMsg { admin: None };
    let mut info = mock_info("alice", &[]);
    instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let sender = Addr::unchecked("alice").to_string();
    let recipient = Addr::unchecked("bob").to_string();
    let amount = Uint128::new(100);
    let start_time = mock_env().block.time.plus_seconds(100).seconds();
    let end_time = mock_env().block.time.plus_seconds(200).seconds();

    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender,
        amount,
        msg: to_binary(&ReceiveMsg::CreateStream {
            admin: None,
            recipient,
            start_time,
            end_time,
            is_detachable: None,
        })
        .unwrap(),
    });
    info.sender = Addr::unchecked("wrongCw20");
    let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
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
    let mut deps = mock_dependencies();
    let sender = Addr::unchecked("alice").to_string();
    let info = mock_info(&sender, &[]);
    instantiate(
        deps.as_mut(),
        mock_env(),
        info.clone(),
        InstantiateMsg { admin: None },
    )
    .unwrap();

    let recipient = Addr::unchecked("bob").to_string();
    let amount = Uint128::new(350);
    let env = mock_env();
    let start_time = env.block.time.plus_seconds(100).seconds();
    let end_time = env.block.time.plus_seconds(400).seconds();

    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: sender.clone(),
        amount,
        msg: to_binary(&ReceiveMsg::CreateStream {
            admin: Some(sender.clone()),
            recipient,
            start_time,
            end_time,
            is_detachable: None,
        })
        .unwrap(),
    });
    execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let stream_id: StreamId = 1;

    //Remove stream and verify not found error returned
    let remove_response = execute(
        deps.as_mut(),
        mock_env(),
        info,
        ExecuteMsg::RemoveStream { id: stream_id },
    )
    .unwrap();
    let error = get_stream(deps.as_ref(), stream_id).unwrap_err();
    assert_eq!(
        error,
        StdError::NotFound {
            kind: "cw_payroll::state::Stream".to_string()
        }
    );

    let refund_msg = remove_response.messages[0].clone().msg;
    assert_eq!(
        refund_msg,
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: sender.clone(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: sender,
                amount
            })
            .unwrap(),
            funds: vec![]
        })
    );
}
#[test]
fn test_execute_link_stream_invalid() {
    let mut deps = mock_dependencies();
    let sender = Addr::unchecked("alice").to_string();

    let info = mock_info(&sender, &[]);

    instantiate(
        deps.as_mut(),
        mock_env(),
        info.clone(),
        InstantiateMsg { admin: None },
    )
    .unwrap();

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
    execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let ids = vec![1, 2];

    //Link stream and verify error returned
    let error = execute(
        deps.as_mut(),
        mock_env(),
        info.clone(),
        ExecuteMsg::LinkStream { ids: ids.clone() },
    )
    .unwrap_err();
    assert_eq!(
        error,
        ContractError::StreamNotFound {
            stream_id: *ids.second().unwrap()
        }
    );

    let ids = vec![1, 1];

    //Link stream and verify error returned
    let error = execute(
        deps.as_mut(),
        mock_env(),
        info.clone(),
        ExecuteMsg::LinkStream { ids },
    )
    .unwrap_err();
    assert_eq!(error, ContractError::StreamsShouldNotBeEqual {});

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
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let sender = Addr::unchecked("bob").to_string();
    let ids = vec![1, 2];

    let unauthorized_info = mock_info(&sender, &[]);
    let error = execute(
        deps.as_mut(),
        mock_env(),
        unauthorized_info,
        ExecuteMsg::LinkStream { ids },
    )
    .unwrap_err();
    assert_eq!(error, ContractError::Unauthorized {});
}

#[test]
fn test_execute_link_stream_valid() {
    let mut deps = mock_dependencies();
    let sender = Addr::unchecked("alice").to_string();

    let info = mock_info(&sender, &[]);

    instantiate(
        deps.as_mut(),
        mock_env(),
        info.clone(),
        InstantiateMsg { admin: None },
    )
    .unwrap();

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
    execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

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
    execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    let ids = vec![1, 2];
    let response = execute(
        deps.as_mut(),
        mock_env(),
        info,
        ExecuteMsg::LinkStream { ids: ids.clone() },
    )
    .unwrap();

    let left_stream = get_stream(deps.as_ref(), *ids.first().unwrap()).unwrap();
    let right_stream = get_stream(deps.as_ref(), *ids.second().unwrap()).unwrap();
    assert_eq!(left_stream.link_id, Some(*ids.second().unwrap()));
    assert_eq!(right_stream.link_id, Some(*ids.first().unwrap()));
    assert!(response
        .attributes
        .iter()
        .any(|f| { f.key == "left_stream_id" }));
    assert!(response
        .attributes
        .iter()
        .any(|f| { f.key == "right_stream_id" }));
}

#[test]
fn test_execute_detach_stream_valid() {
    let mut deps = mock_dependencies();
    let sender = Addr::unchecked("alice").to_string();

    let info = mock_info(&sender, &[]);

    instantiate(
        deps.as_mut(),
        mock_env(),
        info.clone(),
        InstantiateMsg { admin: None },
    )
    .unwrap();

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
    execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

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
    execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    let ids = vec![1, 2];

    execute(
        deps.as_mut(),
        mock_env(),
        info.clone(),
        ExecuteMsg::LinkStream { ids: ids.clone() },
    )
    .unwrap();

    execute(
        deps.as_mut(),
        mock_env(),
        info,
        ExecuteMsg::DetachStream {
            id: *ids.first().unwrap(),
        },
    )
    .unwrap();
    let left_stream = get_stream(deps.as_ref(), *ids.first().unwrap()).unwrap();
    let right_stream = get_stream(deps.as_ref(), *ids.second().unwrap()).unwrap();

    assert!(left_stream.paused);
    assert!(left_stream.paused_time.is_some());
    assert!(left_stream.link_id.is_none());

    assert!(right_stream.paused);
    assert!(right_stream.paused_time.is_some());
    assert!(right_stream.link_id.is_none());
}

#[test]
fn test_execute_detach_stream_invalid() {
    let mut deps = mock_dependencies();
    let sender = Addr::unchecked("alice").to_string();

    let info = mock_info(&sender, &[]);

    instantiate(
        deps.as_mut(),
        mock_env(),
        info.clone(),
        InstantiateMsg { admin: None },
    )
    .unwrap();

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
            is_detachable: Some(false),
        })
        .unwrap(),
    });
    execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    //Create stream 2
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: sender.clone(),
        amount,
        msg: to_binary(&ReceiveMsg::CreateStream {
            admin: Some(sender.clone()),
            recipient: recipient.clone(),
            start_time,
            end_time,
            is_detachable: Some(false),
        })
        .unwrap(),
    });
    execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    let ids = vec![1, 2];

    execute(
        deps.as_mut(),
        mock_env(),
        info.clone(),
        ExecuteMsg::LinkStream { ids },
    )
    .unwrap();
    let ids = vec![11, 22];

    let error = execute(
        deps.as_mut(),
        mock_env(),
        info.clone(),
        ExecuteMsg::DetachStream {
            id: *ids.first().unwrap(),
        },
    )
    .unwrap_err();

    assert_eq!(
        error,
        ContractError::StreamNotFound {
            stream_id: *ids.first().unwrap()
        }
    );
    let ids = vec![1, 22];

    let error = execute(
        deps.as_mut(),
        mock_env(),
        info.clone(),
        ExecuteMsg::DetachStream {
            id: *ids.second().unwrap(),
        },
    )
    .unwrap_err();

    assert_eq!(
        error,
        ContractError::StreamNotFound {
            stream_id: *ids.second().unwrap()
        }
    );
    let ids = vec![1, 2];
    let error = execute(
        deps.as_mut(),
        mock_env(),
        info.clone(),
        ExecuteMsg::DetachStream {
            id: *ids.second().unwrap(),
        },
    )
    .unwrap_err();

    assert_eq!(error, ContractError::StreamNotDetachable {});

    let unauthorized_info = mock_info(&recipient, &[]);

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
    execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    //Create stream 2
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: sender.clone(),
        amount,
        msg: to_binary(&ReceiveMsg::CreateStream {
            admin: Some(sender),
            recipient,
            start_time,
            end_time,
            is_detachable: Some(true),
        })
        .unwrap(),
    });
    execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    let ids = vec![3, 4];
    execute(
        deps.as_mut(),
        mock_env(),
        info,
        ExecuteMsg::LinkStream { ids: ids.clone() },
    )
    .unwrap();

    let error = execute(
        deps.as_mut(),
        mock_env(),
        unauthorized_info,
        ExecuteMsg::DetachStream {
            id: *ids.first().unwrap(),
        },
    )
    .unwrap_err();

    assert_eq!(error, ContractError::Unauthorized {});
}

#[test]
fn test_execute_resume_stream_valid() {
    let mut deps = mock_dependencies();
    let sender = Addr::unchecked("alice").to_string();

    let info = mock_info(&sender, &[]);

    instantiate(
        deps.as_mut(),
        mock_env(),
        info.clone(),
        InstantiateMsg { admin: None },
    )
    .unwrap();

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
    execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

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
    execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    let ids = vec![1, 2];

    execute(
        deps.as_mut(),
        mock_env(),
        info.clone(),
        ExecuteMsg::LinkStream { ids: ids.clone() },
    )
    .unwrap();

    execute(
        deps.as_mut(),
        mock_env(),
        info.clone(),
        ExecuteMsg::DetachStream {
            id: *ids.first().unwrap(),
        },
    )
    .unwrap();
    let left_stream = get_stream(deps.as_ref(), *ids.first().unwrap()).unwrap();
    let right_stream = get_stream(deps.as_ref(), *ids.second().unwrap()).unwrap();

    assert!(left_stream.paused);
    assert!(left_stream.paused_time.is_some());
    assert!(left_stream.link_id.is_none());

    assert!(right_stream.paused);
    assert!(right_stream.paused_time.is_some());
    assert!(right_stream.link_id.is_none());

    execute(
        deps.as_mut(),
        mock_env(),
        info,
        ExecuteMsg::ResumeStream {
            id: *ids.first().unwrap(),
        },
    )
    .unwrap();
    let left_stream = get_stream(deps.as_ref(), *ids.first().unwrap()).unwrap();
    let right_stream = get_stream(deps.as_ref(), *ids.second().unwrap()).unwrap();

    assert!(!left_stream.paused);
    assert!(left_stream.paused_time.is_none());

    assert!(right_stream.paused);
    assert!(right_stream.paused_time.is_some());
}

#[test]
fn test_execute_resume_stream_invalid() {
    let mut deps = mock_dependencies();
    let sender = Addr::unchecked("alice").to_string();

    let info = mock_info(&sender, &[]);

    instantiate(
        deps.as_mut(),
        mock_env(),
        info.clone(),
        InstantiateMsg { admin: None },
    )
    .unwrap();

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
    execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

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
    execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let ids = vec![1, 2];
    execute(
        deps.as_mut(),
        mock_env(),
        info.clone(),
        ExecuteMsg::LinkStream { ids },
    )
    .unwrap();
    let ids = vec![1, 2];

    let error = execute(
        deps.as_mut(),
        mock_env(),
        info,
        ExecuteMsg::ResumeStream {
            id: *ids.second().unwrap(),
        },
    )
    .unwrap_err();
    assert_eq!(error, ContractError::StreamNotPaused {});

    let ids = vec![1, 2];
    let unauthorized_info = mock_info(&recipient, &[]);
    let error = execute(
        deps.as_mut(),
        mock_env(),
        unauthorized_info,
        ExecuteMsg::DetachStream {
            id: *ids.first().unwrap(),
        },
    )
    .unwrap_err();

    assert_eq!(error, ContractError::StreamNotDetachable {});
}
