#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_binary, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Order, Response, StdError,
    StdResult, Storage, Uint128,
};
use cw2::set_contract_version;
use cw20::Cw20ReceiveMsg;
use cw_denom::UncheckedDenom;
use cw_storage_plus::Bound;

use crate::error::ContractError;
use crate::linking::*;
use crate::msg::{
    ConfigResponse, ExecuteMsg, InstantiateMsg, ListStreamsResponse, QueryMsg, ReceiveMsg,
    StreamParams, StreamResponse,
};
use crate::state::{Config, Stream, StreamId, CONFIG, STREAMS, STREAM_SEQ};

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

    // TODO what is the point of this admin? It doesn't do anything.
    // Delete if it doesn't do anything
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
        .add_attribute("admin", admin))
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
        // TODO should be able to create and fund with a native token
        ExecuteMsg::Create { .. } => unimplemented!(),
        ExecuteMsg::Distribute { id } => execute_distribute(env, deps, id),
        ExecuteMsg::PauseStream { id } => execute_pause_stream(env, deps, info, id),
        ExecuteMsg::ResumeStream { id } => execute_resume_stream(env, deps, info, id),
        ExecuteMsg::RemoveStream { id } => execute_remove_stream(env, deps, info, id),
        ExecuteMsg::LinkStream { ids } => execute_link_stream(deps, &info, ids),
        ExecuteMsg::DetachStream { id } => execute_detach_stream(env, deps, &info, id),
    }
}

pub fn execute_pause_stream(
    env: Env,
    deps: DepsMut,
    info: MessageInfo,
    id: StreamId,
) -> Result<Response, ContractError> {
    let pause_stream_local =
        |stream_id: StreamId, storage: &mut dyn Storage| -> Result<Stream, ContractError> {
            let mut stream = STREAMS
                .may_load(storage, stream_id)?
                .ok_or(ContractError::StreamNotFound { stream_id: id })?;
            if stream.admin != info.sender {
                return Err(ContractError::Unauthorized {});
            }
            if stream.paused {
                return Err(ContractError::StreamAlreadyPaused {});
            }
            stream.paused_time = Some(env.block.time.seconds());
            stream.paused = true;
            STREAMS.save(storage, id, &stream)?;
            Ok(stream)
        };

    // TODO this is weird... needs comments at the very least
    let stream = pause_stream_local(id, deps.storage)?;
    if let Some(link_id) = stream.link_id {
        pause_stream_local(link_id, deps.storage)?;
    }

    Ok(Response::new()
        .add_attribute("method", "pause_stream")
        .add_attribute("paused", stream.paused.to_string())
        .add_attribute("stream_id", id.to_string())
        .add_attribute("admin", info.sender)
        .add_attribute("paused_time", stream.paused_time.unwrap().to_string()))
}

pub fn execute_remove_stream(
    env: Env,
    deps: DepsMut,
    info: MessageInfo,
    id: StreamId,
) -> Result<Response, ContractError> {
    // Check that sender is admin
    let stream = STREAMS
        .may_load(deps.storage, id)?
        .ok_or(ContractError::StreamNotFound { stream_id: id })?;
    if stream.admin != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    if let Some(link_id) = stream.link_id {
        return Err(ContractError::LinkedStreamDeleteNotAllowed { link_id });
    }
    STREAMS.remove(deps.storage, id);

    // Transfer any remaining balance to the owner
    let transfer_to_admin_msg = stream
        .denom
        .get_transfer_to_message(&stream.admin, stream.balance)?;

    Ok(Response::new()
        .add_attribute("method", "remove_stream")
        .add_attribute("stream_id", id.to_string())
        .add_attribute("admin", info.sender)
        .add_attribute("removed_time", env.block.time.to_string())
        .add_message(transfer_to_admin_msg))
}

pub fn execute_resume_stream(
    env: Env,
    deps: DepsMut,
    info: MessageInfo,
    id: StreamId,
) -> Result<Response, ContractError> {
    let resume_stream_local =
        |stream_id: StreamId, storage: &mut dyn Storage| -> Result<Stream, ContractError> {
            let mut stream = STREAMS
                .may_load(storage, stream_id)?
                .ok_or(ContractError::StreamNotFound { stream_id: id })?;
            if stream.admin != info.sender {
                return Err(ContractError::Unauthorized {});
            }
            if !stream.paused {
                return Err(ContractError::StreamNotPaused {});
            }
            stream.paused_duration = stream.calc_pause_duration(env.block.time);
            stream.paused = false;
            stream.paused_time = None;
            STREAMS.save(storage, id, &stream)?;
            Ok(stream)
        };

    let stream = resume_stream_local(id, deps.storage)?;
    if let Some(link_id) = stream.link_id {
        resume_stream_local(link_id, deps.storage).unwrap();
    }

    let (_, rate_per_second) = stream.calc_distribution_rate(env.block.time)?;
    let response = Response::new()
        .add_attribute("method", "resume_stream")
        .add_attribute("stream_id", id.to_string())
        .add_attribute("admin", info.sender)
        .add_attribute("rate_per_second", rate_per_second)
        .add_attribute("resume_time", env.block.time.to_string())
        .add_attribute(
            "paused_duration",
            stream.paused_duration.unwrap().to_string(),
        )
        .add_attribute("resume_time", env.block.time.to_string());

    Ok(response)
}

pub fn execute_create_stream(
    env: Env,
    deps: DepsMut,
    params: StreamParams,
) -> Result<Response, ContractError> {
    let StreamParams {
        admin,
        recipient,
        balance,
        denom,
        start_time,
        end_time,
        title,
        description,
        is_detachable,
    } = params;

    let admin = deps.api.addr_validate(&admin)?;
    let recipient = deps.api.addr_validate(&recipient)?;

    if start_time > end_time {
        return Err(ContractError::InvalidStartTime {});
    }

    let block_time = env.block.time.seconds();

    if end_time <= block_time {
        return Err(ContractError::InvalidEndTime {});
    }

    let stream = Stream {
        admin: admin.clone(),
        recipient: recipient.clone(),
        balance,
        claimed_balance: Uint128::zero(),
        denom,
        start_time,
        end_time,
        paused_time: None,
        paused_duration: None,
        paused: false,
        title,
        description,
        link_id: None,
        // TODO Should not be an option type
        is_detachable: is_detachable.unwrap_or(true),
    };

    let id = STREAM_SEQ.load(deps.storage)?;
    let id = id + 1;
    STREAM_SEQ.save(deps.storage, &id)?;
    STREAMS.save(deps.storage, id, &stream)?;

    Ok(Response::new()
        .add_attribute("method", "create_stream")
        .add_attribute("stream_id", id.to_string())
        .add_attribute("admin", admin)
        .add_attribute("recipient", recipient)
        .add_attribute("start_time", start_time.to_string())
        .add_attribute("end_time", end_time.to_string()))
}

pub fn execute_receive(
    env: Env,
    deps: DepsMut,
    info: MessageInfo,
    receive_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    deps.api.addr_validate(&info.sender.clone().into_string())?;
    let msg: ReceiveMsg = from_binary(&receive_msg.msg)?;
    let checked_denom =
        UncheckedDenom::Cw20(info.sender.to_string()).into_checked(deps.as_ref())?;

    // TODO should support all stream params
    match msg {
        ReceiveMsg::CreateStream {
            admin,
            start_time,
            end_time,
            recipient,
            is_detachable,
        } => execute_create_stream(
            env,
            deps,
            StreamParams {
                admin: admin.unwrap_or_else(|| receive_msg.sender.clone()),
                recipient,
                balance: receive_msg.amount,
                denom: checked_denom,
                start_time,
                end_time,
                title: None,
                description: None,
                is_detachable,
            },
        ),
    }
}

pub fn execute_distribute(env: Env, deps: DepsMut, id: u64) -> Result<Response, ContractError> {
    let mut stream = STREAMS
        .may_load(deps.storage, id)?
        .ok_or(ContractError::StreamNotFound { stream_id: id })?;

    let (available_claims, _) = stream.calc_distribution_rate(env.block.time)?;

    if !stream.can_distribute_more() || available_claims.u128() == 0 {
        return Err(ContractError::NoFundsToClaim {
            claimed: stream.claimed_balance,
        });
    }

    // Update claimed amount
    stream.claimed_balance = stream
        .claimed_balance
        .checked_add(available_claims)
        .map_err(StdError::overflow)?;

    // Update remaining balance
    stream.balance = stream
        .balance
        .checked_sub(available_claims)
        .map_err(StdError::overflow)?;

    // Save updated stream
    STREAMS.save(deps.storage, id, &stream)?;

    // Get transfer message
    let transfer_msg = stream
        .denom
        .get_transfer_to_message(&stream.recipient, available_claims)?;

    Ok(Response::new()
        .add_attribute("method", "distribute")
        .add_attribute("vested", available_claims)
        .add_attribute("stream_id", id.to_string())
        .add_message(transfer_msg))
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
        denom: stream.denom,
        start_time: stream.start_time,
        end_time: stream.end_time,
        title: stream.title,
        description: stream.description,
        paused_time: stream.paused_time,
        paused_duration: stream.paused_duration,
        paused: stream.paused,
        is_detachable: stream.is_detachable,
        link_id: stream.link_id,
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
        denom: stream.denom,
        start_time: stream.start_time,
        end_time: stream.end_time,
        title: stream.title,
        description: stream.description,
        paused_time: stream.paused_time,
        paused_duration: stream.paused_duration,
        paused: stream.paused,
        is_detachable: stream.is_detachable,
        link_id: stream.link_id,
    })
}
