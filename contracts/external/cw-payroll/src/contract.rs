use std::borrow::BorrowMut;
use std::default;
use std::sync::mpsc::SendError;

use crate::balance::*;
use crate::error::ContractError;
use crate::msg::{
    ConfigResponse, ExecuteMsg, InstantiateMsg, ListStreamsResponse, QueryMsg, ReceiveMsg,
    StreamId, StreamParams, StreamResponse,
};
use crate::state::{
    add_stream, remove_stream, save_stream, Config, Stream, CONFIG, STREAMS, STREAM_SEQ,
};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_binary, to_binary, Addr, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env,
    MessageInfo, Order, Response, StdResult, Uint128,
};
use cw2::set_contract_version;
use cw20::{Balance, Cw20CoinVerified, Cw20Contract, Cw20ExecuteMsg, Cw20ReceiveMsg};
use cw_storage_plus::Bound;

const CONTRACT_NAME: &str = "crates.io:cw-escrow-streams";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let admin = match msg.admin {
        Some(ad) => deps.api.addr_validate(&ad)?,
        None => info.sender,
    };

    let config = Config {
        admin: admin.clone(),
    };
    CONFIG.save(deps.storage, &config)?;

    STREAM_SEQ.save(deps.storage, &0u64)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", admin))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Receive(msg) => execute_receive(env, deps, info, msg),
        ExecuteMsg::Distribute { id } => execute_distribute(env, deps, info, id),
        ExecuteMsg::PauseStream { id } => execute_pause_stream(env, deps, info, id),
        ExecuteMsg::ResumeStream {
            id,
            start_time,
            end_time,
        } => execute_resume_stream(env, deps, info, id, start_time, end_time),
        ExecuteMsg::RemoveStream { id } => execute_remove_stream(env, deps, info, id),
    }
}
pub fn execute_pause_stream(
    env: Env,
    deps: DepsMut,
    info: MessageInfo,
    id: StreamId,
) -> Result<Response, ContractError> {
    let mut stream = STREAMS
        .may_load(deps.storage, id)?
        .ok_or(ContractError::StreamNotFound {})?;

    if stream.admin != info.sender {
        return Err(ContractError::Unauthorized {});
    }
    if stream.paused {
        return Err(ContractError::StreamAlreadyPaused {});
    }
    stream.paused_time = Some(env.block.time.seconds());
    stream.paused = true;
    save_stream(deps, id, &stream).unwrap();
    let response = Response::new()
        .add_attribute("method", "pause_stream")
        .add_attribute("paused", stream.paused.to_string())
        .add_attribute("stream_id", id.to_string())
        .add_attribute("admin", info.sender)
        .add_attribute("paused_time", stream.paused_time.unwrap().to_string());

    Ok(response)
}
pub fn execute_remove_stream(
    env: Env,
    deps: DepsMut,
    info: MessageInfo,
    id: StreamId,
) -> Result<Response, ContractError> {
    let stream = STREAMS
        .may_load(deps.storage, id)?
        .ok_or(ContractError::StreamNotFound {})?;

    if stream.admin != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    remove_stream(deps, id).unwrap();
    let response = Response::new()
        .add_attribute("method", "remove_stream")
        .add_attribute("stream_id", id.to_string())
        .add_attribute("admin", info.sender)
        .add_attribute("removed_time", env.block.time.to_string());

    Ok(response)
}
pub fn execute_resume_stream(
    env: Env,
    deps: DepsMut,
    info: MessageInfo,
    id: StreamId,
    _start_time: Option<u64>,
    _end_time: Option<u64>,
) -> Result<Response, ContractError> {
    let mut stream = STREAMS
        .may_load(deps.storage, id)?
        .ok_or(ContractError::StreamNotFound {})?;

    if stream.admin != info.sender {
        return Err(ContractError::Unauthorized {});
    }
    if !stream.paused {
        return Err(ContractError::StreamNotPaused {});
    }
    let duration: u128 = (stream.end_time - stream.start_time).into();
    let paused_duration: u128 = (env.block.time.seconds() - stream.paused_time.unwrap()).into();
    let rate_per_second = Uint128::from(stream.balance.amount() / duration - paused_duration);
    stream.paused = false;
    stream.rate_per_second = rate_per_second;

    save_stream(deps, id, &stream).unwrap();
    let response = Response::new()
        .add_attribute("method", "resume_stream")
        .add_attribute("stream_id", id.to_string())
        .add_attribute("admin", info.sender)
        .add_attribute("rate_per_second", rate_per_second)
        .add_attribute("paused_time", stream.paused_time.unwrap().to_string())
        .add_attribute("resume_time", env.block.time.to_string());

    Ok(response)
}
pub fn execute_create_stream(
    env: Env,
    deps: DepsMut,
    _info: MessageInfo,
    params: StreamParams,
) -> Result<Response, ContractError> {
    let StreamParams {
        admin,
        recipient,
        balance,
        start_time,
        end_time,
        title,
        description,
        paused_time,
        paused,
    } = params;

    let owner = deps.api.addr_validate(&admin)?;
    let recipient = deps.api.addr_validate(&recipient)?;

    if start_time > end_time {
        return Err(ContractError::InvalidStartTime {});
    }

    let block_time = env.block.time.seconds();
    if start_time <= block_time && end_time <= block_time {
        return Err(ContractError::InvalidStartTime {});
    }

    if end_time < block_time {
        return Err(ContractError::InvalidEndTime {});
    }

    let duration: u128 = (end_time - start_time).into();
    let mut msgs: Vec<CosmosMsg> = vec![];
    let balance_amount = balance.amount();
    let refund: u128 = balance_amount
        .checked_rem(duration)
        .ok_or(ContractError::Overflow {})?;

    if balance_amount < duration {
        return Err(ContractError::AmountLessThanDuration {});
    }
    if let Some(native_balance) = balance.native() {
        if refund > 0 {
            let msg = CosmosMsg::Bank(BankMsg::Send {
                to_address: owner.clone().into(),
                amount: vec![Coin {
                    denom: native_balance.denom.clone(),
                    amount: native_balance.amount,
                }],
            });
            msgs.push(msg);
        }
    } else if let Some(cw20_balance) = balance.cw20() {
        if refund > 0 {
            let cw20 = Cw20Contract(cw20_balance.address.clone());
            let msg = cw20.call(Cw20ExecuteMsg::Transfer {
                recipient: owner.clone().into(),
                amount: refund.into(),
            })?;
            msgs.push(msg);
        }
    }

    let rate_per_second = Uint128::from(balance_amount.checked_sub(duration).unwrap_or_default());
    let claimed= if balance.is_native() {
        WrappedBalance::default()
    } else {
        WrappedBalance::new_cw20(balance.cw20().unwrap().address.clone(),Uint128::new(0))
    };
    let stream = Stream {
        admin: owner.clone(),
        recipient: recipient.clone(),
        balance,
        claimed_balance:claimed,
        start_time,
        end_time,
        rate_per_second,
        paused_time,
        paused,
        title,
        description,
    };
    let id = add_stream(deps, &stream)?;

    let mut response = Response::new()
        .add_attribute("method", "create_stream")
        .add_attribute("stream_id", id.to_string())
        .add_attribute("admin", admin.clone())
        .add_attribute("recipient", recipient)
        .add_attribute("start_time", start_time.to_string())
        .add_attribute("end_time", end_time.to_string());

    if !msgs.is_empty() {
        response = response.add_messages(msgs);
    }
    Ok(response)
}

pub fn execute_receive(
    env: Env,
    deps: DepsMut,
    info: MessageInfo,
    wrapped: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    deps.api.addr_validate(&info.sender.clone().into_string())?;
    let msg: ReceiveMsg = from_binary(&wrapped.msg)?;
    match msg {
        ReceiveMsg::CreateStream {
            admin,
            start_time,
            end_time,
            recipient,
        } => execute_create_stream(
            env,
            deps,
            info,
            StreamParams {
                admin: admin.unwrap_or(wrapped.sender.clone()),
                recipient,
                balance: WrappedBalance::from(wrapped),
                start_time,
                end_time,
                title: None,
                description: None,
                paused_time: None,
                paused: false,
            },
        ),
    }
}

pub fn execute_distribute(
    env: Env,
    deps: DepsMut,
    info: MessageInfo,
    id: u64,
) -> Result<Response, ContractError> {
    let mut stream = STREAMS
        .may_load(deps.storage, id)?
        .ok_or(ContractError::StreamNotFound {})?;

    if stream.recipient != info.sender {
        return Err(ContractError::NotStreamRecipient {
            recipient: stream.recipient,
        });
    }
    if !stream.can_ditribute_more() {
        return Err(ContractError::NoFundsToClaim {});
    }

    let block_time = env.block.time.seconds();
    let time_passed = std::cmp::min(block_time, stream.end_time).saturating_sub(stream.start_time);
    let mut msgs: Vec<CosmosMsg> = vec![];

    if let Some(native_balance) = stream.balance.native() {
        let vested = Coin {
            denom: native_balance.denom.clone(),
            amount: Uint128::from(time_passed) * stream.rate_per_second,
        };
        let claimed = stream.claimed_balance.amount();
        let released = vested.amount.u128() - claimed;

        if released == 0 {
            return Err(ContractError::NoFundsToClaim {});
        }
        stream
            .claimed_balance
            .checked_add_native(&[vested.clone()])
            .unwrap();
        let msg = CosmosMsg::Bank(BankMsg::Send {
            to_address: stream.recipient.clone().into(),
            amount: vec![Coin {
                denom: native_balance.denom.clone(),
                amount: native_balance.amount,
            }],
        });
        msgs.push(msg);
        stream.balance.checked_sub_native(&vec![vested]).unwrap();
    } else if let Some(cw20) = stream.balance.cw20() {
        let vested = Cw20CoinVerified {
            address: cw20.address.clone(),
            amount: Uint128::from(time_passed) * stream.rate_per_second,
        };
        let claimed = stream.claimed_balance.amount();
        let released = vested.amount.u128() - claimed;
        if released == 0 {
            return Err(ContractError::NoFundsToClaim {});
        }
        stream
            .claimed_balance
            .checked_add_cw20(&[vested.clone()])
            .unwrap();

        let cw20 = Cw20Contract(cw20.address.clone());
        let msg = cw20.call(Cw20ExecuteMsg::Transfer {
            recipient: stream.recipient.clone().into(),
            amount: released.into(),
        })?;
        msgs.push(msg);
        stream.balance.checked_sub_cw20(&[vested]).unwrap();
    }

    STREAMS.save(deps.storage, id, &stream)?;

    let mut res = Response::new()
        .add_attribute("method", "withdraw")
        .add_attribute("stream_id", id.to_string());
    if !msgs.is_empty() {
        res = res.add_messages(msgs);
    }
    Ok(res)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetConfig {} => to_binary(&query_config(deps)?),
        QueryMsg::GetStream { id } => to_binary(&query_stream(deps, id)?),
        QueryMsg::ListStreams { start, limit } => {
            to_binary(&query_list_streams(deps, start, limit)?)
        }
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        admin: config.admin.into(),
    })
}

fn query_stream(deps: Deps, id: u64) -> StdResult<StreamResponse> {
    let stream = STREAMS.load(deps.storage, id)?;
    Ok(StreamResponse {
        id,
        admin: stream.admin.into(),
        recipient: stream.recipient.into(),
        balance: stream.balance,
        claimed_balance: stream.claimed_balance,
        rate_per_second: stream.rate_per_second,
        start_time: stream.start_time,
        end_time: stream.end_time,
        title: stream.title,
        description: stream.description,
        paused_time: stream.paused_time,
        paused: stream.paused,
    })
}

fn query_list_streams(
    deps: Deps,
    start: Option<u8>,
    limit: Option<u8>,
) -> StdResult<ListStreamsResponse> {
    let start = start.map(Bound::inclusive);
    let limit = limit.unwrap_or(5);

    let streams = STREAMS
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit.into())
        .map(map_stream)
        .collect::<StdResult<Vec<_>>>()?;
    Ok(ListStreamsResponse { streams })
}

fn map_stream(item: StdResult<(u64, Stream)>) -> StdResult<StreamResponse> {
    item.map(|(id, stream)| StreamResponse {
        id,
        admin: stream.admin.to_string(),
        recipient: stream.recipient.to_string(),
        balance: stream.balance,
        claimed_balance: stream.claimed_balance,
        start_time: stream.start_time,
        end_time: stream.end_time,
        rate_per_second: stream.rate_per_second,
        title: stream.title,
        description: stream.description,
        paused_time: stream.paused_time,
        paused: stream.paused,
    })
}

#[cfg(test)]
mod tests {

    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{Addr, CosmosMsg, WasmMsg};

    fn get_stream(deps: Deps, id: u64) -> Stream {
        let msg = QueryMsg::GetStream { id };
        let res = query(deps, mock_env(), msg).unwrap();
        from_binary(&res).unwrap()
    }

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
    fn test_execute_withdraw() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg { admin: None };
        let info = mock_info("cw20", &[]);
        instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        let sender = Addr::unchecked("alice").to_string();
        let recipient = Addr::unchecked("bob").to_string();
        let amount = Uint128::new(200);

        let balance = WrappedBalance::new_cw20(Addr::unchecked("cw20"), amount);
        let claimed = WrappedBalance::new_cw20(Addr::unchecked("cw20"), Uint128::new(0));
        let env = mock_env();
        let start_time = env.block.time.plus_seconds(100).seconds();
        let end_time = env.block.time.plus_seconds(300).seconds();

        let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: sender.clone(),
            amount,
            msg: to_binary(&ReceiveMsg::CreateStream {
                admin: Some(sender.clone()),
                recipient,
                start_time,
                end_time,
            })
            .unwrap(),
        });
        execute(deps.as_mut(), env.clone(), info, msg).unwrap();

        assert_eq!(
            get_stream(deps.as_ref(), 1),
            Stream {
                admin: Addr::unchecked("alice"),
                recipient: Addr::unchecked("bob"),
                balance: balance.clone(),
                claimed_balance: claimed,
                start_time,
                rate_per_second: Uint128::new(0),
                end_time,
                title: None,
                description: None,
                paused: false,
                paused_time: None,
            }
        );

        // Stream has not started
        let mut info = mock_info("owner", &[]);
        info.sender = Addr::unchecked("bob");
        let msg = ExecuteMsg::Distribute { id: 1 };
        let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert_eq!(err, ContractError::NoFundsToClaim {});

        // Stream has started so tokens have vested
        let msg = ExecuteMsg::Distribute { id: 1 };
        let mut info = mock_info("owner", &[]);
        let mut env = mock_env();
        let sender = Addr::unchecked("bob");
        info.sender = sender.clone();
        env.block.time = env.block.time.plus_seconds(150);
        let res = execute(deps.as_mut(), env, info, msg).unwrap();
        let msg = res.messages[0].clone().msg;

        assert_eq!(
            msg,
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: String::from("cw20"),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: String::from("bob"),
                    amount: Uint128::new(50)
                })
                .unwrap(),
                funds: vec![]
            })
        );

        assert_eq!(
            get_stream(deps.as_ref(), 1),
            Stream {
                admin: Addr::unchecked("alice"),
                recipient: Addr::unchecked("bob"),
                balance: balance,
                claimed_balance: WrappedBalance::new_cw20(sender, Uint128::new(50)),
                start_time,
                rate_per_second: Uint128::new(1),
                end_time,
                title: None,
                description: None,
                paused: false,
                paused_time: None,
            }
        );

        // Stream has ended so claim remaining tokens

        let mut env = mock_env();
        env.block.time = env.block.time.plus_seconds(500);
        let mut info = mock_info("owner", &[]);
        info.sender = Addr::unchecked("bob");
        let msg = ExecuteMsg::Distribute { id: 1 };
        let res = execute(deps.as_mut(), env, info, msg).unwrap();
        let msg = res.messages[0].clone().msg;

        assert_eq!(
            msg,
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: String::from("cw20"),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: String::from("bob"),
                    amount: Uint128::new(150)
                })
                .unwrap(),
                funds: vec![]
            })
        );
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
                    recipient: sender.clone(),
                    amount: Uint128::new(50)
                })
                .unwrap(),
                funds: vec![]
            })
        );
        let balance_amount=Uint128::new(350);
        let duration: u128 = (end_time - start_time).into();
        let rate_per_second = Uint128::from(balance_amount.u128().checked_sub(duration).unwrap_or_default());

        assert_eq!(
            get_stream(deps.as_ref(), 1),
            Stream {
                admin: sender_addr.clone(),
                recipient: Addr::unchecked("bob"),
                balance: WrappedBalance::new_cw20(Addr::unchecked("cw20"), balance_amount), // original amount - refund
                claimed_balance: WrappedBalance::new_cw20(Addr::unchecked("cw20"), Uint128::new(0)),
                start_time,
                rate_per_second,
                end_time,
                title: None,
                description: None,
                paused: false,
                paused_time: None,
            }
        );
    }
    #[test]
    fn test_execute_pause_stream() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg { admin: None };

        let sender = Addr::unchecked("alice").to_string();
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
        let saved_stream = get_stream(deps.as_ref(), stream_id);
        assert_eq!(
            saved_stream,
            Stream {
                admin: Addr::unchecked("alice"),
                recipient: Addr::unchecked("bob"),
                balance: WrappedBalance::new_cw20(Addr::unchecked("alice"), amount), // original amount - refund
                claimed_balance: WrappedBalance::new_cw20(Addr::unchecked("cw20"), Uint128::new(0)),
                start_time,
                rate_per_second: Uint128::new(0),
                end_time,
                title: None,
                description: None,
                paused: true,
                paused_time: Some(env.block.time.seconds()),
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

    #[test]
    fn test_invalid_deposit_amount() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg { admin: None };
        let mut info = mock_info("alice", &[]);
        instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        let sender = Addr::unchecked("alice").to_string();
        let recipient = Addr::unchecked("bob").to_string();
        let amount = Uint128::new(3);
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
            })
            .unwrap(),
        });
        info.sender = Addr::unchecked("cw20");
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(err, ContractError::AmountLessThanDuration {});
    }
}
