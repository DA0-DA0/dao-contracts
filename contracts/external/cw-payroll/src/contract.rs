use std::borrow::BorrowMut;
use std::default;
use std::sync::mpsc::SendError;

use crate::balance::GenericBalance;
use crate::error::ContractError;
use crate::msg::{
    ConfigResponse, ExecuteMsg, InstantiateMsg, ListStreamsResponse, QueryMsg, ReceiveMsg,
    StreamId, StreamParams, StreamResponse,
};
use crate::state::{save_stream, Config, Stream, CONFIG, STREAMS, STREAM_SEQ};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_binary, to_binary, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo,
    Order, Response, StdResult, Uint128,
};
use cw2::set_contract_version;
use cw20::{Balance, Cw20CoinVerified, Cw20Contract, Cw20ExecuteMsg, Cw20ReceiveMsg};
use cw_storage_plus::Bound;

const CONTRACT_NAME: &str = "crates.io:cw20-streams";
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
        } => execute_pause_stream(env, deps, info, id),
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
    save_stream(deps, &stream).unwrap();
    let response = Response::new()
        .add_attribute("method", "create_stream")
        .add_attribute("stream_id", id.to_string())
        .add_attribute("admin", info.sender)
        .add_attribute("paused_time", env.block.time.to_string());

    Ok(response)
}
pub fn execute_remove_stream(
    env: Env,
    deps: DepsMut,
    info: MessageInfo,
    id: StreamId,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    todo!();
}
pub fn execute_resume_stream(
    env: Env,
    deps: DepsMut,
    info: MessageInfo,
    id: StreamId,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    todo!();
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
    // TODO: Change to support previous dates, within given reason?
    //       The problem with this is it restricts backdating,
    //       so all funds must vest before distribute (for good or bad)
    // TODO: start_time Could default to now?
    // UPD: Fixed
    if start_time <= block_time && end_time <= block_time {
        return Err(ContractError::InvalidStartTime {});
    }

    if end_time < block_time {
        return Err(ContractError::InvalidEndTime {});
    }

    let duration: u128 = (end_time - start_time).into();
    let total_amount = Uint128::default();
    let mut msgs: Vec<CosmosMsg> = vec![];
    if !balance.native.is_empty() {
        for coin in balance.native.iter() {
            let refund: u128 = coin
                .amount
                .u128()
                .checked_rem(duration)
                .ok_or(ContractError::Overflow {})?;

            if coin.amount.u128() < duration {
                return Err(ContractError::AmountLessThanDuration {});
            }
            total_amount.checked_add(coin.amount).unwrap();
            if refund > 0 {
                // TODO: Change to support native and fix cw20
                let msg = CosmosMsg::Bank(BankMsg::Send {
                    to_address: owner.clone().into(),
                    amount: vec![Coin {
                        denom: coin.denom.clone(),
                        amount: coin.amount,
                    }],
                });
                msgs.push(msg);
            }
        }
    }
    if !balance.cw20.is_empty() {
        for coin in balance.cw20.iter() {
            let refund: u128 = coin
                .amount
                .u128()
                .checked_rem(duration)
                .ok_or(ContractError::Overflow {})?;

            if coin.amount.u128() < duration {
                return Err(ContractError::AmountLessThanDuration {});
            }
            total_amount.checked_add(coin.amount).unwrap();
            if refund > 0 {
                // TODO: Change to support native and fix cw20
                let cw20 = Cw20Contract(coin.address.clone());
                let msg = cw20.call(Cw20ExecuteMsg::Transfer {
                    recipient: owner.clone().into(),
                    amount: refund.into(),
                })?;
                msgs.push(msg);
            }
        }
    }
    // Duration must divide evenly into amount, so refund remainder
    // TODO: Change logic to work for cw20 & native

    let rate_per_second = Uint128::from(total_amount.u128() / duration);

    let stream = Stream {
        admin: owner.clone(),
        recipient: recipient.clone(),
        balance,
        claimed_balance: GenericBalance::default(),
        start_time,
        end_time,
        rate_per_second,
        paused_time,
        paused,
        title,
        description,
    };
    let id = save_stream(deps, &stream)?;

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

// TODO: Add native balance recieve for diff streams
pub fn execute_receive(
    env: Env,
    deps: DepsMut,
    info: MessageInfo,
    wrapped: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    // TODO: Change based on asset type found
    //Fixed
    let cw_balance = Balance::Cw20(Cw20CoinVerified {
        address: Clone::clone(&info.sender),
        amount: wrapped.amount,
    });

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
            // TODO: Change this, but still conform to standard
            StreamParams {
                admin: admin.unwrap_or(wrapped.sender),
                recipient,
                balance: cw_balance.into(),
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

    // TODO: Why is this required? is there a security issue here if someone pays to send you tokens all the time?
    // NOTE: commenting out for simple testing with croncat, if required will add approved list.

    // TODO: Def change to better comparitor
    //Fixed

    if stream.recipient != info.sender {
        return Err(ContractError::NotStreamRecipient {
            recipient: stream.recipient,
        });
    }
    if stream.verify_can_ditribute_more() {
        return Err(ContractError::StreamFullyClaimed {});
    }

    let block_time = env.block.time.seconds();
    let time_passed = std::cmp::min(block_time, stream.end_time).saturating_sub(stream.start_time);
    let mut msgs: Vec<CosmosMsg> = vec![];
    let mut vested_list: Vec<Cw20CoinVerified> = vec![];

    for coin in stream.balance.cw20.iter() {
        let vested = Cw20CoinVerified {
            address: coin.address.clone(),
            amount: Uint128::from(time_passed) * stream.rate_per_second,
        };
        let claimed = stream.find_claimed(&coin);
        let claimed_amount = if let Some(cl) = claimed {
            cl.amount
        } else {
            Uint128::MIN
        };
        let released = vested.amount - claimed_amount;

        if released.u128() == 0 {
            return Err(ContractError::NoFundsToClaim {});
        }
        stream
            .claimed_balance
            .checked_add_cw20(&[vested.clone()])
            .unwrap();
        vested_list.push(vested);

        let cw20 = Cw20Contract(coin.address.clone());
        let msg = cw20.call(Cw20ExecuteMsg::Transfer {
            recipient: stream.recipient.clone().into(),
            amount: released,
        })?;
        msgs.push(msg);
    }
    stream.balance.checked_sub_cw20(&vested_list).unwrap();

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
    fn initialization() {
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
    fn execute_withdraw() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg { admin: None };
        let info = mock_info("cw20", &[]);
        instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        let sender = Addr::unchecked("alice").to_string();
        let recipient = Addr::unchecked("bob").to_string();
        let amount = Uint128::new(200);
        let balance = GenericBalance {
            native: vec![Coin {
                denom: String::from("ujunox"),
                amount: amount,
            }],
            cw20: vec![],
        };
        let claimed = GenericBalance {
            native: vec![Coin {
                denom: String::from("ujunox"),
                amount: Uint128::new(0),
            }],
            cw20: vec![],
        };
        let env = mock_env();
        let start_time = env.block.time.plus_seconds(100).seconds();
        let end_time = env.block.time.plus_seconds(300).seconds();

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
        execute(deps.as_mut(), env.clone(), info, msg).unwrap();

        assert_eq!(
            get_stream(deps.as_ref(), 1),
            Stream {
                admin: Addr::unchecked("alice"),
                recipient: Addr::unchecked("bob"),
                balance: balance.clone(),
                claimed_balance: claimed,
                start_time,
                rate_per_second: Uint128::new(1),
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
        info.sender = Addr::unchecked("bob");
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
                claimed_balance: GenericBalance {
                    native: vec![Coin {
                        denom: String::from("ujunox"),
                        amount: Uint128::new(50)
                    }],
                    cw20: vec![]
                },
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
    fn create_stream_with_refund() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg { admin: None };
        let info = mock_info("cw20", &[]);
        instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        let sender = Addr::unchecked("alice").to_string();
        let recipient = Addr::unchecked("bob").to_string();
        let amount = Uint128::new(350);
        let env = mock_env();
        let start_time = env.block.time.plus_seconds(100).seconds();
        let end_time = env.block.time.plus_seconds(400).seconds();

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

        // Make sure remaining funds were refunded if duration didn't divide evenly into amount
        let res = execute(deps.as_mut(), env, info, msg).unwrap();
        let refund_msg = res.messages[0].clone().msg;
        assert_eq!(
            refund_msg,
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: String::from("cw20"),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: String::from("alice"),
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
                balance: GenericBalance {
                    native: vec![Coin {
                        denom: String::from("ujunox"),
                        amount: Uint128::new(300)
                    }],
                    cw20: vec![]
                }, // original amount - refund
                claimed_balance: GenericBalance {
                    native: vec![Coin {
                        denom: String::from("ujunox"),
                        amount: Uint128::new(0)
                    }],
                    cw20: vec![]
                },
                start_time,
                rate_per_second: Uint128::new(1),
                end_time,
                title: None,
                description: None,
                paused: false,
                paused_time: None,
            }
        );
    }

    #[test]
    fn invalid_start_time() {
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
        assert_eq!(err, ContractError::Unauthorized {});
    }

    #[test]
    fn invalid_deposit_amount() {
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
