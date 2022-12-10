use cosmwasm_std::{DepsMut, Env, MessageInfo, Response};

use crate::{
    msg::StreamId,
    state::{save_stream, STREAMS},
    ContractError,
};

pub enum LinkSyncType {
    Paused,
    Resumed,
}
pub(crate) fn execute_link_stream(
    deps: DepsMut,
    info: &MessageInfo,
    left_stream_id: StreamId,
    right_stream_id: StreamId,
) -> Result<Response, ContractError> {
    let left_stream =
        STREAMS
            .may_load(deps.storage, left_stream_id)?
            .ok_or(ContractError::StreamNotFound {
                stream_id: left_stream_id,
            })?;

    let right_stream =
        STREAMS
            .may_load(deps.storage, right_stream_id)?
            .ok_or(ContractError::StreamNotFound {
                stream_id: right_stream_id,
            })?;

    if !(left_stream.admin == info.sender && right_stream.admin == info.sender) {
        return Err(ContractError::Unauthorized {});
    }

    save_stream(deps.storage, left_stream_id, &left_stream).unwrap();
    save_stream(deps.storage, right_stream_id, &right_stream).unwrap();

    let response = Response::new()
        .add_attribute("method", "link")
        .add_attribute("left_stream_id", left_stream_id.to_string())
        .add_attribute("right_stream_id", right_stream_id.to_string())
        .add_attribute("admin", info.sender.clone());

    Ok(response)
}

pub(crate) fn execute_detach_stream(
    env: Env,
    deps: DepsMut,
    info: &MessageInfo,
    left_stream_id: StreamId,
    right_stream_id: StreamId,
) -> Result<Response, ContractError> {
    let mut left_stream = STREAMS.may_load(deps.storage, left_stream_id)?.ok_or(
        ContractError::LinkedStreamNotFound {
            stream_id: left_stream_id,
        },
    )?;
    let mut right_stream = STREAMS.may_load(deps.storage, right_stream_id)?.ok_or(
        ContractError::LinkedStreamNotFound {
            stream_id: right_stream_id,
        },
    )?;

    if !(left_stream.is_detachable && right_stream.is_detachable) {
        return Err(ContractError::StreamNotDetachable {});
    }

    if !(left_stream.admin == info.sender && right_stream.admin == info.sender) {
        return Err(ContractError::Unauthorized {});
    }

    left_stream.paused_time = Some(env.block.time.seconds());
    left_stream.paused = true;
    left_stream.link_id = None;
    right_stream.paused_time = Some(env.block.time.seconds());
    right_stream.paused = true;
    right_stream.link_id = None;

    save_stream(deps.storage, left_stream_id, &left_stream).unwrap();
    save_stream(deps.storage, right_stream_id, &right_stream).unwrap();
    let response = Response::new()
        .add_attribute("method", "link")
        .add_attribute("left_stream_id", left_stream_id.to_string())
        .add_attribute("right_stream_id", right_stream_id.to_string())
        .add_attribute("admin", info.sender.clone());

    Ok(response)
}
