use cosmwasm_std::{Deps, DepsMut, StdResult};
use prost::Message;
use prost_types::FileDescriptorSet;

use crate::{
    state::{ENABLED, NEXT_ID, PROTOBUF_REGISTRY},
    ContractError,
};

/// Ensure the RBAM system is enabled.
pub fn ensure_enabled(deps: DepsMut) -> Result<(), ContractError> {
    if !ENABLED.load(deps.storage)? {
        return Err(ContractError::SystemDisabled {});
    }
    Ok(())
}

/// Gets the next ID and increments state by first incrementing in state and
/// then decrementing the updated return value.
pub fn get_next_id(deps: DepsMut) -> StdResult<u64> {
    NEXT_ID
        // Increment the ID in state
        .update(deps.storage, |id| Ok(id + 1))
        // Decrement the new ID to get the previous ID
        .map(|id| id - 1)
}

/// Gets the file descriptor set bytes for the given protobuf messages by
/// querying the protobuf registry.
pub fn get_encoded_file_descriptor_set(
    deps: &Deps,
    messages: Vec<String>,
) -> Result<Vec<u8>, ContractError> {
    let registry = PROTOBUF_REGISTRY
        .load(deps.storage)
        .map_err(|_| ContractError::MissingProtobufRegistry {})?;
    let res: cw_protobuf_registry::msg::FileDescriptorSetResponse = deps.querier.query_wasm_smart(
        registry,
        &cw_protobuf_registry::msg::QueryMsg::FileDescriptorSet { messages },
    )?;

    Ok(res.file_descriptor_set)
}

/// Gets the file descriptor set for the given protobuf messages by querying the
/// protobuf registry.
pub fn get_file_descriptor_set(
    deps: &Deps,
    messages: Vec<String>,
) -> Result<FileDescriptorSet, ContractError> {
    let fds = get_encoded_file_descriptor_set(deps, messages)?;
    Ok(FileDescriptorSet::decode(fds.as_slice())?)
}
