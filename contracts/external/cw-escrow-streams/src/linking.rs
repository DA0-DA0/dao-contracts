use cosmwasm_std::{DepsMut, Env, MessageInfo, Response};

use crate::{
    state::{StreamId, StreamIds, StreamIdsExtensions, STREAMS},
    ContractError,
};

pub enum LinkSyncType {
    Paused,
    Resumed,
}

pub(crate) fn execute_link_stream(
    deps: DepsMut,
    info: &MessageInfo,
    ids: StreamIds,
) -> Result<Response, ContractError> {
    ids.validate()?;
    // TODO no unwrap
    let left_stream_id = *ids.first().unwrap();
    let right_stream_id = *ids.second().unwrap();

    let mut left_stream =
        STREAMS
            .may_load(deps.storage, left_stream_id)?
            .ok_or(ContractError::StreamNotFound {
                stream_id: *ids.first().unwrap(),
            })?;

    let mut right_stream =
        STREAMS
            .may_load(deps.storage, right_stream_id)?
            .ok_or(ContractError::StreamNotFound {
                stream_id: right_stream_id,
            })?;

    if !(left_stream.owner == info.sender && right_stream.owner == info.sender) {
        return Err(ContractError::Unauthorized {});
    }
    left_stream.link_id = Some(right_stream_id);
    right_stream.link_id = Some(left_stream_id);

    STREAMS.save(deps.storage, left_stream_id, &left_stream)?;
    STREAMS.save(deps.storage, right_stream_id, &right_stream)?;

    let response = Response::new()
        .add_attribute("method", "link")
        .add_attribute("left_stream_id", left_stream_id.to_string())
        .add_attribute("right_stream_id", right_stream_id.to_string())
        .add_attribute("owner", info.sender.clone());

    Ok(response)
}

pub(crate) fn execute_detach_stream(
    env: Env,
    deps: DepsMut,
    info: &MessageInfo,
    id: StreamId,
) -> Result<Response, ContractError> {
    let mut detach_stream = STREAMS
        .may_load(deps.storage, id)?
        .ok_or(ContractError::StreamNotFound { stream_id: id })?;

    let link_id = detach_stream.link_id.unwrap();
    let mut linked_stream = STREAMS
        .may_load(deps.storage, link_id)?
        .ok_or(ContractError::StreamNotFound { stream_id: link_id })?;

    if !(detach_stream.is_detachable && linked_stream.is_detachable) {
        return Err(ContractError::StreamNotDetachable {});
    }

    if !(detach_stream.owner == info.sender && linked_stream.owner == info.sender) {
        return Err(ContractError::Unauthorized {});
    }

    detach_stream.paused_time = Some(env.block.time.seconds());
    detach_stream.paused = true;
    detach_stream.link_id = None;
    linked_stream.paused_time = Some(env.block.time.seconds());
    linked_stream.paused = true;
    linked_stream.link_id = None;

    STREAMS.save(deps.storage, id, &detach_stream)?;
    STREAMS.save(deps.storage, link_id, &linked_stream)?;

    let response = Response::new()
        .add_attribute("method", "link")
        .add_attribute("detach_stream_id", id.to_string())
        .add_attribute("linked_stream_id", link_id.to_string())
        .add_attribute("owner", info.sender.clone());

    Ok(response)
}
