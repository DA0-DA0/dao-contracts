#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Order, Response, StdError, StdResult,
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

    // Default the owner to the sender.
    let owner = msg.owner.map_or_else(
        || Ok(info.sender.clone()),
        |owner| deps.api.addr_validate(&owner),
    )?;
    initialize_owner(deps.storage, deps.api, Some(owner.as_str()))?;

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

fn execute_register(
    deps: DepsMut,
    info: MessageInfo,
    file_descriptor_sets: Vec<Vec<u8>>,
) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    if file_descriptor_sets.is_empty() {
        return Err(ContractError::NoFiles {});
    }

    let mut file_count = 0u32;
    let mut message_count = 0u32;

    for (file_descriptor_set_index, file_descriptor_set) in
        file_descriptor_sets.into_iter().enumerate()
    {
        let file_descriptor_set = FileDescriptorSet::decode(file_descriptor_set.as_slice())?;
        for (new_fd_index, new_fd) in file_descriptor_set.file.into_iter().enumerate() {
            let file_name =
                new_fd
                    .name
                    .clone()
                    .ok_or(ContractError::FileDescriptorMissingName {
                        file_descriptor_index: new_fd_index,
                        file_descriptor_set_index,
                    })?;
            let file_package =
                new_fd
                    .package
                    .clone()
                    .ok_or(ContractError::FileDescriptorMissingPackage {
                        file_descriptor_index: new_fd_index,
                        file_descriptor_name: file_name.clone(),
                        file_descriptor_set_index,
                    })?;

            // Store map of messages' full names to their file names so their
            // files can be looked up later when messages are referenced in
            // filters.
            for (message_descriptor_index, message_descriptor) in
                new_fd.message_type.iter().enumerate()
            {
                let message_type_name = message_descriptor.name.clone().ok_or_else(|| {
                    ContractError::MessageDescriptorMissingName {
                        message_descriptor_index,
                        file_name: file_name.clone(),
                        file_package: file_package.clone(),
                    }
                })?;
                let message_full_name = format!("{}.{}", file_package, message_type_name);

                MESSAGES.save(deps.storage, message_full_name, &file_name)?;

                message_count += 1;
            }

            // Get existing file descriptor data.
            let existing_file_descriptor_data = FILES.may_load(deps.storage, file_name.clone())?;

            // If file is already registered, merge the files.
            if let Some(existing_fd_data) = existing_file_descriptor_data {
                let mut existing_fd = FileDescriptorProto::decode(existing_fd_data.as_slice())?;

                // If the file package changed, error.
                if existing_fd.package.as_ref() != Some(&file_package) {
                    return Err(ContractError::FileDescriptorPackageChanged {
                        file_name,
                        file_package: existing_fd.package.clone().unwrap_or_default(),
                        new_file_package: file_package,
                    });
                }

                // Add new or overwrite existing dependencies/messages/enums.

                for dependency in new_fd.dependency {
                    if !existing_fd.dependency.contains(&dependency) {
                        existing_fd.dependency.push(dependency);
                    }
                }

                for mut new_message in new_fd.message_type {
                    if let Some(existing_message) = existing_fd
                        .message_type
                        .iter_mut()
                        .find(|m| m.name == new_message.name)
                    {
                        // Replace with new message.
                        std::mem::swap(existing_message, &mut new_message);
                    } else {
                        // Add new message.
                        existing_fd.message_type.push(new_message);
                    }
                }

                for mut new_enum in new_fd.enum_type {
                    if let Some(existing_enum_type) = existing_fd
                        .enum_type
                        .iter_mut()
                        .find(|e| e.name == new_enum.name)
                    {
                        // Replace with new enum.
                        std::mem::swap(existing_enum_type, &mut new_enum);
                    } else {
                        // Add new enum.
                        existing_fd.enum_type.push(new_enum);
                    }
                }

                FILES.save(deps.storage, file_name, &existing_fd.encode_to_vec())?;
            } else {
                let file_descriptor_data = new_fd.encode_to_vec();
                FILES.save(deps.storage, file_name, &file_descriptor_data)?;
            }

            file_count += 1;
        }
    }

    Ok(Response::new()
        .add_attribute("action", "register")
        .add_attribute("file_count", file_count.to_string())
        .add_attribute("message_count", message_count.to_string()))
}

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

fn execute_prepare(
    deps: DepsMut,
    info: MessageInfo,
    messages: Vec<String>,
) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    for message in messages {
        let fds = create_file_descriptor_set_for_messages(&deps.as_ref(), &[message.clone()])?
            .encode_to_vec();
        PREPARED.save(deps.storage, message, &fds)?;
    }

    Ok(Response::new().add_attribute("action", "prepare"))
}

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
        if messages.len() == 1 && PREPARED.has(deps.storage, messages[0].clone()) {
            PREPARED.load(deps.storage, messages[0].clone())?
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
    let file_descriptor_set =
        create_file_descriptor_set_for_messages(&deps, &[message_name.clone()]).map_err(|e| {
            StdError::generic_err(format!("failed to create file descriptor set: {}", e))
        })?;

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
