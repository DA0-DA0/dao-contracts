#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Order, Response, StdError, StdResult,
    Storage,
};
use cw2::set_contract_version;
use cw_ownable::initialize_owner;
use cw_storage_plus::Bound;
use cw_utils::nonpayable;
use dao_interface::proposal::InfoResponse;
use prost::Message;
use prost_reflect::{DescriptorPool, DynamicMessage};
use prost_types::{FileDescriptorProto, FileDescriptorSet};

use crate::error::ContractError;
use crate::msg::{
    DecodeResponse, ExecuteMsg, FileDescriptorSetResponse, InstantiateMsg, ListFilesResponse,
    ListMessagesResponse, MigrateMsg, PreparedResponse, QueryMsg,
};
use crate::protobuf::create_file_descriptor_set_for_messages;
use crate::state::{FILES, MESSAGES, PREPARED};

pub(crate) const CONTRACT_NAME: &str = "crates.io:cw-protobuf-registry";
pub(crate) const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const DEFAULT_LIMIT: u32 = 30;
const MAX_LIMIT: u32 = 100;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Default the owner to the sender if unset
    let owner = &match msg.owner {
        Some(addr) => addr,
        None => info.sender.to_string(),
    };

    // initialize_owner call performs the addr validation
    initialize_owner(deps.storage, deps.api, Some(owner))?;

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("creator", info.sender.as_str())
        .add_attribute("owner", owner.as_str()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    match msg {
        ExecuteMsg::UpdateOwnership(action) => execute_update_ownership(deps, info, env, action),
        ExecuteMsg::Register {
            file_descriptor_sets,
        } => execute_register(deps, info, file_descriptor_sets),
        ExecuteMsg::Unregister {
            file_names,
            message_limit,
        } => execute_unregister(deps, info, file_names, message_limit),
        ExecuteMsg::Prepare { messages } => execute_prepare(deps, info, messages),
        ExecuteMsg::Unprepare { messages } => execute_unprepare(deps, info, messages),
    }
}

fn execute_update_ownership(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    action: cw_ownable::Action,
) -> Result<Response, ContractError> {
    let ownership = cw_ownable::update_ownership(deps, &env.block, &info.sender, action)?;
    Ok(Response::new()
        .add_attribute("action", "update_ownership")
        .add_attributes(ownership.into_attributes()))
}

/// accepts a list of FileDescriptorSet objects, which are the
/// compiled output of .proto files.
/// For each file descriptor, it stores the raw descriptor and
/// indexes all the message types within it.
/// If a file with the same name already exists, it merges the new
/// definitions with the old ones. This allows for updating existing
/// protobuf definitions.
fn execute_register(
    deps: DepsMut,
    info: MessageInfo,
    file_descriptor_sets: Vec<Vec<u8>>,
) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    if file_descriptor_sets.is_empty() {
        return Err(ContractError::NoFiles {});
    }

    // counters for response attributes
    let mut file_count = 0u32;
    let mut message_count = 0u32;

    // iterate over each encoded file descriptor set
    for (fds_i, fds_bytes) in file_descriptor_sets.into_iter().enumerate() {
        // decode the file descriptor bytes into a vec of FileDescriptorProto and
        // attempt to register the result
        let file_descriptor_set = FileDescriptorSet::decode(fds_bytes.as_slice())?;
        try_register_fds(
            deps.storage,
            file_descriptor_set.file,
            fds_i,
            &mut file_count,
            &mut message_count,
        )?;
    }

    Ok(Response::new()
        .add_attribute("action", "register")
        .add_attribute("file_count", file_count.to_string())
        .add_attribute("message_count", message_count.to_string()))
}

/// registers descriptor set files.
fn try_register_fds(
    store: &mut dyn Storage,
    fds_files: Vec<FileDescriptorProto>,
    fds_i: usize,
    file_count: &mut u32,
    message_count: &mut u32,
) -> Result<(), ContractError> {
    // iterate over each of the provided file descriptor set files
    for (new_fd_index, new_fd) in fds_files.into_iter().enumerate() {
        let file_name = new_fd
            .name
            .clone()
            .ok_or(ContractError::FileDescriptorMissingName {
                file_descriptor_index: new_fd_index,
                file_descriptor_set_index: fds_i,
            })?;
        let file_package =
            new_fd
                .package
                .clone()
                .ok_or(ContractError::FileDescriptorMissingPackage {
                    file_descriptor_index: new_fd_index,
                    file_descriptor_name: file_name.clone(),
                    file_descriptor_set_index: fds_i,
                })?;

        // Store map of messages' full names to their file names so their
        // files can be looked up later when messages are referenced in
        // filters.
        for (message_descriptor_index, descriptor) in new_fd.message_type.iter().enumerate() {
            let message_type_name = descriptor.name.as_ref().ok_or_else(|| {
                ContractError::MessageDescriptorMissingName {
                    message_descriptor_index,
                    file_name: file_name.to_string(),
                    file_package: file_package.to_string(),
                }
            })?;
            let message_full_name = format!("{}.{}", file_package, message_type_name);

            MESSAGES.save(store, message_full_name, &file_name)?;

            *message_count += 1;
        }

        match FILES.may_load(store, file_name.to_string())? {
            // If file is already registered, merge the files.
            Some(existing_fd_data) => {
                merge_file_descriptors(store, existing_fd_data, file_package, file_name, new_fd)?
            }
            // otherwise, write a new file descriptor under the given file name
            None => {
                let file_descriptor_data = new_fd.encode_to_vec();
                FILES.save(store, file_name, &file_descriptor_data)?;
            }
        }

        *file_count += 1;
    }

    Ok(())
}

fn merge_file_descriptors(
    store: &mut dyn Storage,
    existing_fd_data: Vec<u8>,
    file_package: String,
    file_name: String,
    new_fd: FileDescriptorProto,
) -> Result<(), ContractError> {
    let mut existing_fd = FileDescriptorProto::decode(existing_fd_data.as_slice())?;

    // If the file package changed, error.
    if existing_fd.package.as_deref() != Some(file_package.as_str()) {
        return Err(ContractError::FileDescriptorPackageChanged {
            file_name,
            file_package: existing_fd.package.unwrap_or_default(),
            new_file_package: file_package,
        });
    }

    // Add new or overwrite existing dependencies/messages/enums.
    for dep in new_fd.dependency {
        if !existing_fd.dependency.contains(&dep) {
            existing_fd.dependency.push(dep);
        }
    }

    for new_message in new_fd.message_type {
        if let Some(i) = existing_fd
            .message_type
            .iter()
            .position(|m| m.name == new_message.name)
        {
            existing_fd.message_type[i] = new_message; // replace
        } else {
            existing_fd.message_type.push(new_message); // insert
        }
    }

    for new_enum in new_fd.enum_type {
        if let Some(i) = existing_fd
            .enum_type
            .iter()
            .position(|e| e.name == new_enum.name)
        {
            existing_fd.enum_type[i] = new_enum; // replace
        } else {
            existing_fd.enum_type.push(new_enum); // insert
        }
    }

    FILES.save(store, file_name, &existing_fd.encode_to_vec())?;

    Ok(())
}

/// owner can provide a list of file names to remove from the registry.
/// this will also remove all the associated message definitions from
/// the storage.
/// use message_limit parameter to handle cases where a single file
/// contains a large number of messages that can't be all removed
/// in a single transaction due to gas limits.
/// this should function as an inverse of previous execute_register
/// executions.
fn execute_unregister(
    deps: DepsMut,
    info: MessageInfo,
    file_names: Vec<String>,
    message_limit: Option<u32>,
) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    if file_names.is_empty() {
        return Err(ContractError::NoFiles {});
    }

    let message_limit = message_limit.unwrap_or(u32::MAX);
    let mut unregistered_message_count = 0;

    let total_files = file_names.len();
    for (file_index, file_name) in file_names.into_iter().enumerate() {
        let remaining = (message_limit - unregistered_message_count) as usize;

        // If we've reached the limit of messages to unregister before we start
        // unregistering this file, return an error. The message limit must be
        // large enough to unregister all messages in all files except the last
        // one, which can be partially unregistered. This ensures that you can
        // partially unregister messages from a single file in case there are
        // too many to unregister in a single TX, while still minimizing the
        // number of files that are partially unregistered.
        if remaining == 0 {
            return Err(ContractError::MessageLimitReached {
                unregistered: file_index,
                total: total_files,
            });
        }

        let messages = MESSAGES
            .idx
            .file_name
            .prefix(file_name.clone())
            .keys(deps.storage, None, None, Order::Ascending)
            .take(remaining)
            .collect::<StdResult<Vec<_>>>()?;

        // Unregister messages.
        unregistered_message_count += messages.len() as u32;
        for message in messages {
            MESSAGES.remove(deps.storage, message)?;
        }

        // Unregister file.
        FILES.remove(deps.storage, file_name);
    }

    Ok(Response::new()
        .add_attribute("action", "unregister")
        .add_attribute(
            "unregistered_message_count",
            unregistered_message_count.to_string(),
        ))
}

/// eager loading for actual proto decoding.
/// pre-generates and stores the FileDescriptorSet required
/// to decode a specific message, including all its dependencies.
/// this saves gas on the actual decoding, as the FileDescriptorSet
/// does not need to be loaded just-in-time for each query_decode call.
fn execute_prepare(
    deps: DepsMut,
    info: MessageInfo,
    messages: Vec<String>,
) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    for message in messages {
        // Only prepare messages that are not already prepared.
        if !PREPARED.has(deps.storage, message.to_string()) {
            let fds = create_file_descriptor_set_for_messages(&deps.as_ref(), &[message.clone()])?
                .encode_to_vec();
            PREPARED.save(deps.storage, message, &fds)?;
        }
    }

    Ok(Response::new().add_attribute("action", "prepare"))
}

/// allows the owner to remove the pre-generated
/// FileDescriptorSet for a given list of messages
fn execute_unprepare(
    deps: DepsMut,
    info: MessageInfo,
    messages: Vec<String>,
) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    for message in messages {
        PREPARED.remove(deps.storage, message);
    }

    Ok(Response::new().add_attribute("action", "unprepare"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Ownership {} => to_json_binary(&cw_ownable::get_ownership(deps.storage)?),
        QueryMsg::Info {} => to_json_binary(&query_info(deps)?),
        QueryMsg::ListFiles { start_after, limit } => {
            to_json_binary(&query_list_files(deps, start_after, limit)?)
        }
        QueryMsg::ListMessages {
            file_name,
            start_after,
            limit,
        } => to_json_binary(&query_list_messages(deps, file_name, start_after, limit)?),
        QueryMsg::ListPrepared { start_after, limit } => {
            to_json_binary(&query_list_prepared(deps, start_after, limit)?)
        }
        QueryMsg::Prepared { message_name } => to_json_binary(&query_prepared(deps, message_name)?),
        QueryMsg::FileDescriptorSet { messages } => {
            to_json_binary(&query_file_descriptor_set(deps, messages)?)
        }
        QueryMsg::Decode {
            message_name,
            value,
        } => to_json_binary(&query_decode(deps, message_name, value)?),
    }
}

fn query_info(deps: Deps) -> StdResult<InfoResponse> {
    let info = cw2::get_contract_version(deps.storage)?;
    Ok(InfoResponse { info })
}

fn query_list_files(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<ListFilesResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(Bound::exclusive);

    let files = FILES
        .keys(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .collect::<StdResult<Vec<_>>>()?;

    Ok(ListFilesResponse { files })
}

fn query_list_messages(
    deps: Deps,
    file_name: Option<String>,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<ListMessagesResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(Bound::exclusive);

    let keys = if let Some(file_name) = file_name {
        MESSAGES
            .idx
            .file_name
            .prefix(file_name)
            .keys(deps.storage, start, None, Order::Ascending)
    } else {
        MESSAGES.keys(deps.storage, start, None, Order::Ascending)
    };

    let messages = keys.take(limit).collect::<StdResult<Vec<_>>>()?;

    Ok(ListMessagesResponse { messages })
}

pub fn query_list_prepared(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<ListMessagesResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(Bound::exclusive);

    let messages = PREPARED
        .keys(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .collect::<StdResult<Vec<_>>>()?;

    Ok(ListMessagesResponse { messages })
}

fn query_prepared(deps: Deps, message_name: String) -> StdResult<PreparedResponse> {
    let prepared = PREPARED.has(deps.storage, message_name);
    Ok(PreparedResponse { prepared })
}

fn query_file_descriptor_set(
    deps: Deps,
    messages: Vec<String>,
) -> StdResult<FileDescriptorSetResponse> {
    // If one message is provided and it is already prepared, use the prepared
    // file descriptor set. Otherwise, create a file descriptor set for all
    // messages.
    let file_descriptor_set =
        if messages.len() == 1 && PREPARED.has(deps.storage, messages[0].to_string()) {
            PREPARED.load(deps.storage, messages[0].to_string())?
        } else {
            create_file_descriptor_set_for_messages(&deps, &messages)
                .map_err(|e| StdError::generic_err(e.to_string()))?
                .encode_to_vec()
        };

    Ok(FileDescriptorSetResponse {
        file_descriptor_set,
    })
}

fn query_decode(deps: Deps, message_name: String, value: Vec<u8>) -> StdResult<DecodeResponse> {
    // If the message is already prepared, use the prepared file descriptor set.
    // Otherwise, create a file descriptor set for the message.
    let file_descriptor_set = PREPARED
        .may_load(deps.storage, message_name.clone())?
        .map_or_else(
            || {
                create_file_descriptor_set_for_messages(&deps, &[message_name.clone()]).map_err(
                    |e| {
                        StdError::generic_err(format!(
                            "failed to create file descriptor set: {}",
                            e
                        ))
                    },
                )
            },
            |fds| {
                FileDescriptorSet::decode(fds.as_slice())
                    .map_err(|e| StdError::generic_err(e.to_string()))
            },
        )?;

    let pool = DescriptorPool::from_file_descriptor_set(file_descriptor_set).map_err(|e| {
        StdError::generic_err(format!("failed to create descriptor pool from FDS: {}", e))
    })?;

    // should never error since we created the FDS from the message name, but
    // check just in case.
    let message_descriptor = pool.get_message_by_name(&message_name).ok_or_else(|| {
        StdError::generic_err(format!(
            "message descriptor not found for name: {}",
            message_name
        ))
    })?;

    let message = DynamicMessage::decode(message_descriptor, value.as_slice())
        .map_err(|e| StdError::generic_err(e.to_string()))?;

    let json = serde_json::to_value(message).map_err(|e| {
        StdError::generic_err(format!(
            "failed to serialize decoded protobuf value as JSON: {}",
            e
        ))
    })?;

    Ok(DecodeResponse { value: json })
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    // Set contract to version to latest
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}
