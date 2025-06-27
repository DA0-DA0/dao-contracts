use std::collections::HashSet;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Order, Response, StdError,
    StdResult, WasmMsg,
};
use cw_storage_plus::Bound;

use cw2::set_contract_version;
use cw_ownable::initialize_owner;
use cw_utils::nonpayable;
use dao_interface::helpers::OptionalUpdate;
use prost_reflect::prost::Message;
use prost_reflect::prost_types::FileDescriptorSet;

use crate::action::{Action, ActionToExecute};
use crate::error::ContractError;
use crate::helpers::ensure_enabled;
use crate::msg::{
    ActionResponse, Assignment, AuthorizationResponse, DaoResponse, ExecuteMsg,
    InitialAuthorization, InitialRole, InstantiateMsg, IsAssignedRoleResponse, IsEnabledResponse,
    IsMsgAuthorizedByResponse, IsMsgAuthorizedByRoleResponse, IsMsgAuthorizedResponse,
    ListActionsResponse, ListAddressesWithRoleResponse, ListAssignmentsResponse,
    ListAuthorizationsResponse, ListProtobufFilesResponse, ListProtobufMessagesResponse,
    ListRolesForAddressResponse, ListRolesResponse, MigrateMsg, QueryMsg, RoleResponse,
    TestFilterResponse,
};
use crate::role::{Authorization, Role};
use crate::state::{
    ASSIGNMENTS, AUTHORIZATIONS, DAO, ENABLED, LOG, NEXT_ID, PROTOBUF_FILES, PROTOBUF_MESSAGES,
    ROLES,
};

pub(crate) const CONTRACT_NAME: &str = "crates.io:dao-rbam";
pub(crate) const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const DEFAULT_LIMIT: u32 = 30;
const DEFAULT_LIMIT_IS_MSG_AUTHORIZED: u32 = 30;
const MAX_LIMIT: u32 = 100;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Default the DAO to the instantiator.
    let dao = msg.dao.map_or_else(
        || Ok(info.sender.clone()),
        |dao| deps.api.addr_validate(&dao),
    )?;
    DAO.save(deps.storage, &dao)?;

    // Default the owner to the DAO.
    let owner = msg
        .owner
        .map_or_else(|| Ok(dao.clone()), |owner| deps.api.addr_validate(&owner))?;
    initialize_owner(deps.storage, deps.api, Some(owner.as_str()))?;

    // Initialize enabled state (default to true).
    let enabled = msg.enabled.unwrap_or(true);
    ENABLED.save(deps.storage, &enabled)?;

    // Initialize next ID counter.
    NEXT_ID.save(deps.storage, &1)?;

    let mut response = Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("creator", info.sender.as_str())
        .add_attribute("enabled", enabled.to_string());

    // Create initial roles.
    if let Some(initial_roles) = msg.initial_roles {
        for InitialRole {
            name,
            metadata,
            authorizations,
            assignments,
        } in initial_roles
        {
            let role = Role::create(deps.branch(), name, metadata, true)?;
            response = response.add_attribute("role_id", role.id.to_string());

            // Create initial authorizations for role.
            if let Some(authorizations) = authorizations {
                for InitialAuthorization {
                    name,
                    metadata,
                    filter,
                    enabled,
                } in authorizations
                {
                    let enabled = enabled.unwrap_or(true);
                    let authorization = Authorization::create(
                        deps.branch(),
                        role.id,
                        name,
                        metadata,
                        filter,
                        enabled,
                    )?;
                    response =
                        response.add_attribute("authorization_id", authorization.id.to_string());
                }
            }

            // Assign role.
            if let Some(assignments) = assignments {
                for addr in assignments {
                    let addr = deps.api.addr_validate(&addr)?;
                    Role::assign(deps.branch(), &addr, role.id)?;
                }
            }
        }
    }

    Ok(response)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateOwnership(action) => execute_update_ownership(deps, info, env, action),
        ExecuteMsg::UpdateDao { dao } => execute_update_dao(deps, info, dao),
        ExecuteMsg::SetEnabled { enabled } => execute_set_enabled(deps, info, enabled),
        ExecuteMsg::CreateRole {
            name,
            metadata,
            enabled,
            authorizations,
            assignments,
        } => execute_create_role(
            deps,
            info,
            name,
            metadata,
            enabled,
            authorizations,
            assignments,
        ),
        ExecuteMsg::UpdateRole {
            role_id,
            name,
            metadata,
            enabled,
        } => execute_update_role(deps, info, role_id, name, metadata, enabled),
        ExecuteMsg::CreateAuthorization {
            role_id,
            name,
            metadata,
            filter,
            enabled,
        } => execute_create_authorization(deps, info, role_id, name, metadata, filter, enabled),
        ExecuteMsg::UpdateAuthorization {
            authorization_id,
            name,
            metadata,
            filter,
            enabled,
        } => execute_update_authorization(
            deps,
            info,
            authorization_id,
            name,
            metadata,
            filter,
            enabled,
        ),
        ExecuteMsg::Assign { assign } => execute_assign(deps, info, assign),
        ExecuteMsg::Revoke { revoke } => execute_revoke(deps, info, revoke),
        ExecuteMsg::RegisterProtobufs {
            file_descriptor_sets,
        } => execute_register_protobufs(deps, info, file_descriptor_sets),
        ExecuteMsg::UnregisterProtobufs {
            file_names,
            message_limit,
        } => execute_unregister_protobufs(deps, info, file_names, message_limit),
        ExecuteMsg::ExecuteActions { actions } => execute_execute_actions(deps, env, info, actions),
    }
}

fn execute_update_ownership(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    action: cw_ownable::Action,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    let ownership = cw_ownable::update_ownership(deps, &env.block, &info.sender, action)?;
    Ok(Response::new()
        .add_attribute("action", "update_ownership")
        .add_attributes(ownership.into_attributes()))
}

fn execute_update_dao(
    deps: DepsMut,
    info: MessageInfo,
    dao: String,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    let dao = deps.api.addr_validate(&dao)?;
    DAO.save(deps.storage, &dao)?;

    Ok(Response::new()
        .add_attribute("action", "update_dao")
        .add_attribute("dao", dao.to_string()))
}

fn execute_set_enabled(
    deps: DepsMut,
    info: MessageInfo,
    enabled: bool,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    ENABLED.save(deps.storage, &enabled)?;

    Ok(Response::new()
        .add_attribute("action", "set_enabled")
        .add_attribute("enabled", enabled.to_string()))
}

fn execute_create_role(
    mut deps: DepsMut,
    info: MessageInfo,
    name: String,
    metadata: Option<String>,
    enabled: Option<bool>,
    authorizations: Option<Vec<InitialAuthorization>>,
    assignments: Option<Vec<String>>,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    let enabled = enabled.unwrap_or(true);
    let role = Role::create(deps.branch(), name, metadata, enabled)?;

    let mut response = Response::new()
        .add_attribute("action", "create_role")
        .add_attribute("role_id", role.id.to_string())
        .add_attribute("role_name", role.name);

    // Create initial authorizations for role.
    if let Some(authorizations) = authorizations {
        for InitialAuthorization {
            name,
            metadata,
            filter,
            enabled,
        } in authorizations
        {
            let enabled = enabled.unwrap_or(true);
            let authorization =
                Authorization::create(deps.branch(), role.id, name, metadata, filter, enabled)?;
            response = response.add_attribute("authorization_id", authorization.id.to_string());
        }
    }

    // Assign role.
    if let Some(assignments) = assignments {
        for addr in assignments {
            let addr = deps.api.addr_validate(&addr)?;
            Role::assign(deps.branch(), &addr, role.id)?;
        }
    }

    Ok(response)
}

fn execute_update_role(
    deps: DepsMut,
    info: MessageInfo,
    role_id: u64,
    name: Option<String>,
    metadata: OptionalUpdate<String>,
    enabled: Option<bool>,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    let mut role = Role::load(&deps.as_ref(), role_id)?;

    if let Some(name) = name {
        role.name = name;
    }

    metadata.maybe_update(|value| {
        role.metadata = value;
    });

    if let Some(enabled) = enabled {
        role.enabled = enabled;
    }

    role.save(deps)?;

    Ok(Response::new()
        .add_attribute("action", "update_role")
        .add_attribute("role_id", role_id.to_string())
        .add_attribute("role_name", role.name))
}

fn execute_create_authorization(
    deps: DepsMut,
    info: MessageInfo,
    role_id: u64,
    name: String,
    metadata: Option<String>,
    filter: Option<serde_json::Value>,
    enabled: Option<bool>,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    cw_ownable::assert_owner(deps.storage, &info.sender)?;
    // Ensure role exists.
    Role::ensure_exists(&deps.as_ref(), role_id)?;

    let enabled = enabled.unwrap_or(true);
    let authorization = Authorization::create(deps, role_id, name, metadata, filter, enabled)?;

    Ok(Response::new()
        .add_attribute("action", "create_authorization")
        .add_attribute("role_id", authorization.role_id.to_string())
        .add_attribute("authorization_id", authorization.id.to_string())
        .add_attribute("authorization_name", authorization.name))
}

fn execute_update_authorization(
    deps: DepsMut,
    info: MessageInfo,
    authorization_id: u64,
    name: Option<String>,
    metadata: OptionalUpdate<String>,
    filter: OptionalUpdate<serde_json::Value>,
    enabled: Option<bool>,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    let mut authorization = Authorization::load(&deps.as_ref(), authorization_id)?;

    if let Some(name) = name {
        authorization.name = name;
    }

    metadata.maybe_update(|value| {
        authorization.metadata = value;
    });

    filter.maybe_update_result(|value| -> Result<(), ContractError> {
        authorization.filter = value;
        authorization.sync_protobuf_messages(&deps.as_ref())?;
        Ok(())
    })?;

    if let Some(enabled) = enabled {
        authorization.enabled = enabled;
    }

    authorization.save(deps)?;

    Ok(Response::new()
        .add_attribute("action", "update_authorization")
        .add_attribute("authorization_id", authorization.id.to_string())
        .add_attribute("authorization_name", authorization.name))
}

fn execute_assign(
    mut deps: DepsMut,
    info: MessageInfo,
    assign: Vec<Assignment>,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    if assign.is_empty() {
        return Err(ContractError::NoRoles {});
    }

    let mut role_exists: HashSet<u64> = HashSet::new();

    for Assignment { addr, role_id } in assign {
        // Verify role existence once per role.
        if !role_exists.contains(&role_id) {
            Role::ensure_exists(&deps.as_ref(), role_id)?;
            role_exists.insert(role_id);
        }

        let addr = deps.api.addr_validate(&addr)?;

        // Ensure not assigned.
        if Role::is_assigned(&deps.as_ref(), &addr, role_id) {
            return Err(ContractError::RoleAlreadyAssigned {
                addr: addr.to_string(),
                role_id,
            });
        }

        Role::assign(deps.branch(), &addr, role_id)?;
    }

    Ok(Response::new().add_attribute("action", "assign"))
}

fn execute_revoke(
    mut deps: DepsMut,
    info: MessageInfo,
    revoke: Vec<Assignment>,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    if revoke.is_empty() {
        return Err(ContractError::NoRoles {});
    }

    for Assignment { addr, role_id } in revoke {
        let addr = deps.api.addr_validate(&addr)?;

        // Ensure assigned.
        if !Role::is_assigned(&deps.as_ref(), &addr, role_id) {
            return Err(ContractError::RoleNotAssigned {
                addr: addr.to_string(),
                role_id,
            });
        }

        Role::revoke(deps.branch(), &addr, role_id)?;
    }

    Ok(Response::new().add_attribute("action", "revoke"))
}

fn execute_register_protobufs(
    deps: DepsMut,
    info: MessageInfo,
    file_descriptor_sets: Vec<Vec<u8>>,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    if file_descriptor_sets.is_empty() {
        return Err(ContractError::NoFiles {});
    }

    let mut file_count = 0;
    let mut message_count = 0;

    for (file_descriptor_set_index, file_descriptor_set) in file_descriptor_sets.iter().enumerate()
    {
        let file_descriptor_set = FileDescriptorSet::decode(file_descriptor_set.as_slice())?;

        for (file_descriptor_index, file_descriptor) in file_descriptor_set.file.iter().enumerate()
        {
            let file_name =
                file_descriptor
                    .name
                    .clone()
                    .ok_or(ContractError::FileDescriptorMissingName {
                        file_descriptor_index,
                        file_descriptor_set_index,
                    })?;
            let file_package = file_descriptor.package.clone().ok_or(
                ContractError::FileDescriptorMissingPackage {
                    file_descriptor_index,
                    file_descriptor_name: file_name.clone(),
                    file_descriptor_set_index,
                },
            )?;

            // Get all message descriptors in file.
            for (message_descriptor_index, message_descriptor) in
                file_descriptor.message_type.iter().enumerate()
            {
                let message_type_name = message_descriptor.name.clone().ok_or_else(|| {
                    ContractError::MessageDescriptorMissingName {
                        message_descriptor_index,
                        file_name: file_name.clone(),
                        file_package: file_package.clone(),
                    }
                })?;
                let message_full_name = format!("{}.{}", file_package, message_type_name);

                PROTOBUF_MESSAGES.save(deps.storage, message_full_name, &file_name)?;

                message_count += 1;
            }

            let file_descriptor_data = file_descriptor.encode_to_vec();
            PROTOBUF_FILES.save(deps.storage, file_name, &file_descriptor_data)?;

            file_count += 1;
        }
    }

    Ok(Response::new()
        .add_attribute("action", "register_protobuf")
        .add_attribute("file_count", file_count.to_string())
        .add_attribute("message_count", message_count.to_string()))
}

fn execute_unregister_protobufs(
    deps: DepsMut,
    info: MessageInfo,
    file_names: Vec<String>,
    message_limit: Option<u32>,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    if file_names.is_empty() {
        return Err(ContractError::NoFiles {});
    }

    let message_limit = message_limit.unwrap_or(u32::MAX);
    let mut unregistered_message_count = 0;

    for (file_index, file_name) in file_names.iter().enumerate() {
        let remaining = (message_limit - unregistered_message_count) as usize;

        // If we've reached the limit of messages to unregister before we start
        // unregistering this file, return an error. The message limit must be
        // large enough to unregister all messages in all files except the last
        // one, which can be partially unregistered. This ensures that you can
        // partially unregister messages from a single file in case there are
        // too many to unregister in a single TX, while still minimizing the
        // number of files that are partially unregistered.
        if remaining == 0 {
            return Err(ContractError::ProtobufMessageLimitReached {
                unregistered: file_index,
                total: file_names.len(),
            });
        }

        let messages = PROTOBUF_MESSAGES
            .idx
            .file_name
            .prefix(file_name.to_string())
            .keys(deps.storage, None, None, Order::Ascending)
            .take(remaining)
            .collect::<StdResult<Vec<_>>>()?;

        // Unregister messages.
        unregistered_message_count += messages.len() as u32;
        for message in messages {
            PROTOBUF_MESSAGES.remove(deps.storage, message)?;
        }

        // Unregister file.
        PROTOBUF_FILES.remove(deps.storage, file_name.to_string());
    }

    Ok(Response::new()
        .add_attribute("action", "unregister_protobuf")
        .add_attribute(
            "unregistered_message_count",
            unregistered_message_count.to_string(),
        ))
}

fn execute_execute_actions(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    actions: Vec<ActionToExecute>,
) -> Result<Response, ContractError> {
    ensure_enabled(deps.branch())?;

    if actions.is_empty() {
        return Err(ContractError::NoActions {});
    }

    let (msgs, action_ids): (Vec<_>, Vec<_>) = actions
        .into_iter()
        .map(|action| {
            let action = action.initiate(deps.branch(), &env, &info.sender)?;
            Ok((action.msg, action.id.to_string()))
        })
        .collect::<Result<Vec<_>, ContractError>>()?
        .into_iter()
        .unzip();

    // Execute messages via proposal hook.
    let dao = DAO.load(deps.storage)?;
    let dao_execute_msg = WasmMsg::Execute {
        contract_addr: dao.to_string(),
        msg: to_json_binary(&dao_interface::msg::ExecuteMsg::ExecuteProposalHook { msgs })?,
        funds: vec![],
    };

    Ok(Response::new()
        .add_message(dao_execute_msg)
        .add_attribute("action", "execute_actions")
        .add_attribute("rbam", "true")
        .add_attribute("action_ids", action_ids.join(",")))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Ownership {} => to_json_binary(&cw_ownable::get_ownership(deps.storage)?),
        QueryMsg::Dao {} => to_json_binary(&query_get_dao(deps)?),
        QueryMsg::IsEnabled {} => to_json_binary(&query_is_enabled(deps)?),
        QueryMsg::Role { id } => to_json_binary(&query_get_role(deps, id)?),
        QueryMsg::ListRoles { start_after, limit } => {
            to_json_binary(&query_list_roles(deps, start_after, limit)?)
        }
        QueryMsg::Authorization { id } => to_json_binary(&query_get_authorization(deps, id)?),
        QueryMsg::ListAuthorizations { start_after, limit } => {
            to_json_binary(&query_list_authorizations(deps, start_after, limit)?)
        }
        QueryMsg::ListAuthorizationsByRole {
            role_id,
            start_after,
            limit,
        } => to_json_binary(&query_list_authorizations_by_role(
            deps,
            role_id,
            start_after,
            limit,
        )?),
        QueryMsg::IsAssignedRole { addr, role_id } => {
            to_json_binary(&query_is_assigned_role(deps, addr, role_id)?)
        }
        QueryMsg::ListAssignments { start_after, limit } => {
            to_json_binary(&query_list_assignments(deps, start_after, limit)?)
        }
        QueryMsg::ListAddressesWithRole {
            role_id,
            start_after,
            limit,
        } => to_json_binary(&query_list_assignments_by_role(
            deps,
            role_id,
            start_after,
            limit,
        )?),
        QueryMsg::ListRolesForAddress {
            addr,
            start_after,
            limit,
        } => to_json_binary(&query_list_assignments_by_address(
            deps,
            addr,
            start_after,
            limit,
        )?),
        QueryMsg::ListProtobufFiles { start_after, limit } => {
            to_json_binary(&query_list_protobuf_files(deps, start_after, limit)?)
        }
        QueryMsg::ListProtobufMessages { start_after, limit } => {
            to_json_binary(&query_list_protobuf_messages(deps, start_after, limit)?)
        }
        QueryMsg::ListProtobufMessagesByFile {
            file_name,
            start_after,
            limit,
        } => to_json_binary(&query_list_protobuf_messages_by_file(
            deps,
            file_name,
            start_after,
            limit,
        )?),
        QueryMsg::Action { addr, id } => to_json_binary(&query_get_action(deps, addr, id)?),
        QueryMsg::ListActions {
            start_after,
            limit,
            reverse,
        } => to_json_binary(&query_list_actions(deps, start_after, limit, reverse)?),
        QueryMsg::ListActionsByRole {
            role_id,
            start_after,
            limit,
            reverse,
        } => to_json_binary(&query_list_actions_by_role(
            deps,
            role_id,
            start_after,
            limit,
            reverse,
        )?),
        QueryMsg::ListActionsByAuthorization {
            authorization_id,
            start_after,
            limit,
            reverse,
        } => to_json_binary(&query_list_actions_by_authorization(
            deps,
            authorization_id,
            start_after,
            limit,
            reverse,
        )?),
        QueryMsg::ListActionsByAddress {
            addr,
            start_after,
            limit,
            reverse,
        } => to_json_binary(&query_list_actions_by_address(
            deps,
            addr,
            start_after,
            limit,
            reverse,
        )?),
        QueryMsg::IsMsgAuthorized {
            addr,
            msg,
            start_after,
            limit,
        } => to_json_binary(&query_is_msg_authorized(
            deps,
            addr,
            msg,
            start_after,
            limit,
        )?),
        QueryMsg::IsMsgAuthorizedByRole {
            addr,
            role_id,
            msg,
            start_after,
            limit,
        } => to_json_binary(&query_is_msg_authorized_by_role(
            deps,
            addr,
            role_id,
            msg,
            start_after,
            limit,
        )?),
        QueryMsg::IsMsgAuthorizedBy {
            addr,
            role_id,
            authorization_id,
            msg,
        } => to_json_binary(&query_is_msg_authorized_by(
            deps,
            addr,
            role_id,
            authorization_id,
            msg,
        )?),
        QueryMsg::TestFilter { filter, msg } => {
            to_json_binary(&query_test_filter(deps, filter, msg)?)
        }
    }
}

fn query_get_dao(deps: Deps) -> StdResult<DaoResponse> {
    let dao = DAO.load(deps.storage)?;
    Ok(DaoResponse { dao })
}

fn query_is_enabled(deps: Deps) -> StdResult<IsEnabledResponse> {
    let enabled = ENABLED.load(deps.storage)?;
    Ok(IsEnabledResponse { enabled })
}

fn query_get_role(deps: Deps, id: u64) -> StdResult<RoleResponse> {
    let role = Role::load(&deps, id).map_err(|e| StdError::generic_err(e.to_string()))?;
    Ok(RoleResponse { role })
}

fn query_list_roles(
    deps: Deps,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<ListRolesResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(Bound::exclusive);

    let roles = ROLES
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| item.map(|(_, role)| role))
        .collect::<StdResult<Vec<_>>>()?;

    Ok(ListRolesResponse { roles })
}

fn query_get_authorization(deps: Deps, id: u64) -> StdResult<AuthorizationResponse> {
    let authorization =
        Authorization::load(&deps, id).map_err(|e| StdError::generic_err(e.to_string()))?;
    Ok(AuthorizationResponse { authorization })
}

fn query_list_authorizations(
    deps: Deps,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<ListAuthorizationsResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(Bound::exclusive);

    let authorizations = AUTHORIZATIONS
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| item.map(|(_, authorization)| authorization))
        .collect::<StdResult<Vec<_>>>()?;

    Ok(ListAuthorizationsResponse { authorizations })
}

fn query_list_authorizations_by_role(
    deps: Deps,
    role_id: u64,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<ListAuthorizationsResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(Bound::exclusive);

    let authorizations = AUTHORIZATIONS
        .idx
        .role_id
        .prefix(role_id)
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| item.map(|(_, authorization)| authorization))
        .collect::<StdResult<Vec<_>>>()?;

    Ok(ListAuthorizationsResponse { authorizations })
}

fn query_is_assigned_role(
    deps: Deps,
    addr: String,
    role_id: u64,
) -> StdResult<IsAssignedRoleResponse> {
    let addr = deps.api.addr_validate(&addr)?;
    let assigned = Role::is_assigned(&deps, &addr, role_id);
    Ok(IsAssignedRoleResponse { assigned })
}

fn query_list_assignments(
    deps: Deps,
    start_after: Option<(String, u64)>,
    limit: Option<u32>,
) -> StdResult<ListAssignmentsResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after
        .map(|(addr, role_id)| {
            deps.api
                .addr_validate(&addr)
                .map(|addr| Bound::exclusive((addr, role_id)))
        })
        .transpose()?;

    let assignments = ASSIGNMENTS
        .keys(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            item.map(|(addr, role_id)| Assignment {
                addr: addr.to_string(),
                role_id,
            })
        })
        .collect::<StdResult<Vec<_>>>()?;

    Ok(ListAssignmentsResponse { assignments })
}

fn query_list_assignments_by_role(
    deps: Deps,
    role_id: u64,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<ListAddressesWithRoleResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after
        .map(|addr| {
            deps.api
                .addr_validate(&addr)
                .map(|addr| Bound::exclusive((addr, role_id)))
        })
        .transpose()?;

    let addresses = ASSIGNMENTS
        .idx
        .role_id
        .prefix(role_id)
        .keys(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| item.map(|(addr, _)| addr))
        .collect::<StdResult<Vec<_>>>()?;

    Ok(ListAddressesWithRoleResponse { addresses })
}

fn query_list_assignments_by_address(
    deps: Deps,
    addr: String,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<ListRolesForAddressResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let addr = deps.api.addr_validate(&addr)?;
    let start = start_after.map(Bound::exclusive);

    let role_ids = ASSIGNMENTS
        .prefix(addr.clone())
        .keys(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .collect::<StdResult<Vec<_>>>()?;

    Ok(ListRolesForAddressResponse { role_ids })
}

fn query_list_protobuf_files(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<ListProtobufFilesResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(Bound::exclusive);

    let files = PROTOBUF_FILES
        .keys(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .collect::<StdResult<Vec<_>>>()?;

    Ok(ListProtobufFilesResponse { files })
}

fn query_list_protobuf_messages(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<ListProtobufMessagesResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(Bound::exclusive);

    let messages = PROTOBUF_MESSAGES
        .keys(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .collect::<StdResult<Vec<_>>>()?;

    Ok(ListProtobufMessagesResponse { messages })
}

fn query_list_protobuf_messages_by_file(
    deps: Deps,
    file_name: String,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<ListProtobufMessagesResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(Bound::exclusive);

    let messages = PROTOBUF_MESSAGES
        .idx
        .file_name
        .prefix(file_name)
        .keys(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .collect::<StdResult<Vec<_>>>()?;

    Ok(ListProtobufMessagesResponse { messages })
}

fn query_get_action(deps: Deps, addr: String, action_id: u64) -> StdResult<ActionResponse> {
    let addr = deps.api.addr_validate(&addr)?;
    let action = LOG.load(deps.storage, (addr, action_id)).map_err(|_| {
        StdError::generic_err(ContractError::ActionNotFound { id: action_id }.to_string())
    })?;
    Ok(ActionResponse { action })
}

fn query_list_actions(
    deps: Deps,
    start_after: Option<u64>,
    limit: Option<u32>,
    reverse: Option<bool>,
) -> StdResult<ListActionsResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(Bound::exclusive);
    let (min, max, order) = if reverse.unwrap_or(false) {
        (None, start, Order::Descending)
    } else {
        (start, None, Order::Ascending)
    };

    let actions: Vec<Action> = LOG
        .idx
        .action_id
        .range(deps.storage, min, max, order)
        .take(limit)
        .collect::<StdResult<Vec<_>>>()?
        .into_iter()
        .map(|(_, action)| action)
        .collect();

    Ok(ListActionsResponse { actions })
}

fn query_list_actions_by_role(
    deps: Deps,
    role_id: u64,
    start_after: Option<(String, u64)>,
    limit: Option<u32>,
    reverse: Option<bool>,
) -> StdResult<ListActionsResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after
        .map(|(addr, action_id)| {
            deps.api
                .addr_validate(&addr)
                .map(|addr| Bound::exclusive((addr, action_id)))
        })
        .transpose()?;
    let (min, max, order) = if reverse.unwrap_or(false) {
        (None, start, Order::Descending)
    } else {
        (start, None, Order::Ascending)
    };

    let actions: Vec<Action> = LOG
        .idx
        .role_id
        .prefix(role_id)
        .range(deps.storage, min, max, order)
        .take(limit)
        .map(|item| item.map(|(_, action)| action))
        .collect::<StdResult<Vec<_>>>()?;

    Ok(ListActionsResponse { actions })
}

fn query_list_actions_by_authorization(
    deps: Deps,
    authorization_id: u64,
    start_after: Option<(String, u64)>,
    limit: Option<u32>,
    reverse: Option<bool>,
) -> StdResult<ListActionsResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after
        .map(|(addr, authorization_id)| {
            deps.api
                .addr_validate(&addr)
                .map(|addr| Bound::exclusive((addr, authorization_id)))
        })
        .transpose()?;
    let (min, max, order) = if reverse.unwrap_or(false) {
        (None, start, Order::Descending)
    } else {
        (start, None, Order::Ascending)
    };

    let actions: Vec<Action> = LOG
        .idx
        .authorization_id
        .prefix(authorization_id)
        .range(deps.storage, min, max, order)
        .take(limit)
        .map(|item| item.map(|(_, action)| action))
        .collect::<StdResult<Vec<_>>>()?;

    Ok(ListActionsResponse { actions })
}

fn query_list_actions_by_address(
    deps: Deps,
    addr: String,
    start_after: Option<u64>,
    limit: Option<u32>,
    reverse: Option<bool>,
) -> StdResult<ListActionsResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let addr = deps.api.addr_validate(&addr)?;
    let start = start_after.map(Bound::exclusive);
    let (min, max, order) = if reverse.unwrap_or(false) {
        (None, start, Order::Descending)
    } else {
        (start, None, Order::Ascending)
    };

    let actions: Vec<Action> = LOG
        .prefix(addr)
        .range(deps.storage, min, max, order)
        .take(limit)
        .collect::<StdResult<Vec<_>>>()?
        .into_iter()
        .map(|(_, action)| action)
        .collect();

    Ok(ListActionsResponse { actions })
}

fn query_is_msg_authorized(
    deps: Deps,
    addr: String,
    msg: CosmosMsg,
    start_after: Option<(u64, u64)>,
    limit: Option<u32>,
) -> StdResult<IsMsgAuthorizedResponse> {
    let addr = deps.api.addr_validate(&addr)?;
    let mut remaining_to_check = limit.unwrap_or(DEFAULT_LIMIT_IS_MSG_AUTHORIZED) as usize;

    // Determine which role to start checking from.
    let start_role = start_after.map(|(role_id, _)| Bound::exclusive(role_id));
    // Determine which authorization to start checking from for the first role.
    // This will be reset once we move on to the next role since the order of
    // authorization IDs between two roles is not guaranteed. Since the order of
    // role IDs will always increment, we only need to bound the authorizations
    // query inside each role loop for the first role, as we want to check all
    // authorizations for all future roles.
    let mut start_auth = start_after.map(|(_, auth_id)| Bound::exclusive(auth_id));

    // Get roles assigned to this address.
    let assigned_roles =
        ASSIGNMENTS
            .prefix(addr.clone())
            .keys(deps.storage, start_role, None, Order::Ascending);

    let mut last_checked: Option<(u64, u64)> = None;

    for role_id in assigned_roles {
        let role_id = role_id?;
        last_checked = Some((
            role_id,
            last_checked.map(|(_, auth_id)| auth_id).unwrap_or_default(),
        ));

        // Should not error since this role ID is assigned.
        let role = Role::load(&deps, role_id).map_err(|e| StdError::generic_err(e.to_string()))?;
        // Skip if the role is disabled.
        if !role.enabled {
            continue;
        }

        // Get authorizations for this role.
        let authorizations = AUTHORIZATIONS.idx.role_id.prefix(role.id).range(
            deps.storage,
            start_auth,
            None,
            Order::Ascending,
        );

        for result in authorizations {
            // If we've reached the limit in the past iteration and we have
            // another to check, return error. This ensures we only ever return
            // "limit reached" if we haven't checked all authorizations. If we
            // happen to run out of the limit on the last authorization, and it
            // doesn't match, we don't want to return "limit reached".
            if remaining_to_check == 0 {
                return Ok(IsMsgAuthorizedResponse::Unauthorized {
                    reason: ContractError::LimitReached {}.to_string(),
                    last_checked,
                });
            }

            let (_, authorization) = result?;
            // Update last_checked to the current authorization so the next
            // query knows where to start from. Make sure this happens after the
            // limit check.
            last_checked = Some((role.id, authorization.id));
            remaining_to_check -= 1;

            // Skip if the authorization is disabled.
            if !authorization.enabled {
                continue;
            }

            // Check if the authorization allows the message.
            let allowed = authorization
                .allows(&deps, &msg, true)
                // Should not happen since we ignore filter errors.
                .map_err(|e| StdError::generic_err(e.to_string()))?;

            if allowed {
                return Ok(IsMsgAuthorizedResponse::Authorized {
                    role,
                    authorization,
                });
            }
        }

        // Reset start_auth to None if we've reached the end of the
        // authorizations for this role since the order of authorizations
        // between two roles is not guaranteed, whereas the order of role IDs
        // will always increment. start_auth was only needed for the first role.
        start_auth = None;
    }

    Ok(IsMsgAuthorizedResponse::Unauthorized {
        reason: ContractError::NoMoreAuthorizations {}.to_string(),
        last_checked,
    })
}

fn query_is_msg_authorized_by_role(
    deps: Deps,
    addr: String,
    role_id: u64,
    msg: CosmosMsg,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<IsMsgAuthorizedByRoleResponse> {
    let addr = deps.api.addr_validate(&addr)?;
    let mut remaining_to_check = limit.unwrap_or(DEFAULT_LIMIT_IS_MSG_AUTHORIZED) as usize;
    let start = start_after.map(Bound::exclusive);

    // Ensure the role exists.
    let role = match Role::load(&deps, role_id) {
        Ok(role) => role,
        Err(e) => {
            return Ok(IsMsgAuthorizedByRoleResponse::Unauthorized {
                reason: e.to_string(),
                last_checked: None,
            })
        }
    };

    // Ensure the address is assigned the role.
    let assigned = Role::is_assigned(&deps, &addr, role_id);
    if !assigned {
        return Ok(IsMsgAuthorizedByRoleResponse::Unauthorized {
            reason: ContractError::RoleNotAssigned {
                addr: addr.to_string(),
                role_id,
            }
            .to_string(),
            last_checked: None,
        });
    }

    // Ensure role is enabled.
    if !role.enabled {
        return Ok(IsMsgAuthorizedByRoleResponse::Unauthorized {
            reason: ContractError::RoleDisabled {}.to_string(),
            last_checked: None,
        });
    }

    // Get authorizations for this role.
    let authorizations = AUTHORIZATIONS.idx.role_id.prefix(role_id).range(
        deps.storage,
        start,
        None,
        Order::Ascending,
    );

    let mut last_checked: Option<u64> = None;

    for result in authorizations {
        // If we've reached the limit in the past iteration and we have another
        // to check, return error. This ensures we only ever return "limit
        // reached" if we haven't checked all authorizations. If we happen to
        // run out of the limit on the last authorization, and it doesn't match,
        // we don't want to return "limit reached".
        if remaining_to_check == 0 {
            return Ok(IsMsgAuthorizedByRoleResponse::Unauthorized {
                reason: ContractError::LimitReached {}.to_string(),
                last_checked,
            });
        }

        let (_, authorization) = result?;
        // Update last_checked to the current authorization so the next query
        // knows where to start from. Make sure this happens after the limit
        // check.
        last_checked = Some(authorization.id);
        remaining_to_check -= 1;

        // Skip if the authorization is disabled.
        if !authorization.enabled {
            continue;
        }

        // Check if the authorization allows the message.
        let allowed = authorization
            .allows(&deps, &msg, true)
            // Should not happen since we ignore filter errors.
            .map_err(|e| StdError::generic_err(e.to_string()))?;

        if allowed {
            return Ok(IsMsgAuthorizedByRoleResponse::Authorized {
                role,
                authorization,
            });
        }
    }

    Ok(IsMsgAuthorizedByRoleResponse::Unauthorized {
        reason: ContractError::NoMoreAuthorizations {}.to_string(),
        last_checked,
    })
}

fn query_is_msg_authorized_by(
    deps: Deps,
    addr: String,
    role_id: u64,
    authorization_id: u64,
    msg: CosmosMsg,
) -> StdResult<IsMsgAuthorizedByResponse> {
    let addr = deps.api.addr_validate(&addr)?;

    // Ensure the role and authorization exist.
    let role = match Role::load(&deps, role_id) {
        Ok(role) => role,
        Err(e) => {
            return Ok(IsMsgAuthorizedByResponse::Unauthorized {
                reason: e.to_string(),
            });
        }
    };
    let authorization = match Authorization::load(&deps, authorization_id) {
        Ok(authorization) => authorization,
        Err(e) => {
            return Ok(IsMsgAuthorizedByResponse::Unauthorized {
                reason: e.to_string(),
            });
        }
    };

    // Ensure the authorization belongs to the role.
    if authorization.role_id != role_id {
        return Ok(IsMsgAuthorizedByResponse::Unauthorized {
            reason: ContractError::AuthorizationRoleMismatch {}.to_string(),
        });
    }

    // Ensure address has the role assigned.
    let assigned = Role::is_assigned(&deps, &addr, role_id);
    if !assigned {
        return Ok(IsMsgAuthorizedByResponse::Unauthorized {
            reason: ContractError::RoleNotAssigned {
                addr: addr.to_string(),
                role_id,
            }
            .to_string(),
        });
    }

    // Ensure role is enabled.
    if !role.enabled {
        return Ok(IsMsgAuthorizedByResponse::Unauthorized {
            reason: ContractError::RoleDisabled {}.to_string(),
        });
    }

    // Ensure authorization is enabled.
    if !authorization.enabled {
        return Ok(IsMsgAuthorizedByResponse::Unauthorized {
            reason: ContractError::AuthorizationDisabled {}.to_string(),
        });
    }

    // Check if the authorization allows the message.
    let allowed = match authorization.allows(&deps, &msg, false) {
        Ok(allowed) => allowed,
        Err(e) => {
            return Ok(IsMsgAuthorizedByResponse::Unauthorized {
                reason: e.to_string(),
            });
        }
    };

    if allowed {
        return Ok(IsMsgAuthorizedByResponse::Authorized {
            role,
            authorization,
        });
    }

    Ok(IsMsgAuthorizedByResponse::Unauthorized {
        reason: ContractError::MsgNotAllowedByFilter {
            err: "unknown reason".to_string(),
        }
        .to_string(),
    })
}

fn query_test_filter(
    deps: Deps,
    filter: serde_json::Value,
    msg: CosmosMsg,
) -> StdResult<TestFilterResponse> {
    let check = Authorization::filter_allows(&deps, &filter, None, &msg, false);

    // Handle filter errors appropriately.
    let response = check.map_or_else(
        |e| TestFilterResponse::Fail {
            reason: e.to_string(),
        },
        |allowed| match allowed {
            true => TestFilterResponse::Pass {},
            // should never happen since ignore_filter_error is false
            false => TestFilterResponse::Fail {
                reason: ContractError::MsgNotAllowedByFilter {
                    err: "unknown reason".to_string(),
                }
                .to_string(),
            },
        },
    );

    Ok(response)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    // Set contract to version to latest
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}
