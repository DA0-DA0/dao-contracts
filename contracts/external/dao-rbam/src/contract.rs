use std::collections::HashSet;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, to_json_binary, Attribute, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Order,
    Reply, Response, StdError, StdResult, SubMsg, WasmMsg,
};
use cw_storage_plus::Bound;

use cw2::set_contract_version;
use cw_ownable::initialize_owner;
use cw_utils::{nonpayable, parse_reply_instantiate_data};
use dao_interface::helpers::OptionalUpdate;
use dao_interface::proposal::InfoResponse;
use dao_interface::state::ModuleUpdate;

use crate::action::{Action, ActionToExecute};
use crate::error::ContractError;
use crate::helpers::{ensure_enabled, submsg_instantiate_filter, submsg_instantiate_registry};
use crate::msg::{
    ActionResponse, AssignedResponse, Assignment, AuthorizationResponse, AuthorizedByResponse,
    AuthorizedByRoleResponse, AuthorizedResponse, DaoResponse, EnabledResponse, ExecuteMsg,
    FilterResponse, InitialAuthorization, InitialRole, InstantiateMsg, ListActionsResponse,
    ListAddressesWithRoleResponse, ListAssignmentsResponse, ListAuthorizationsResponse,
    ListRolesForAddressResponse, ListRolesResponse, MigrateMsg, ProtobufRegistryResponse, QueryMsg,
    RoleResponse, TestFilterResponse,
};
use crate::role::{Authorization, Role};
use crate::state::{
    PendingFilterInstall, ASSIGNMENTS, AUTHORIZATIONS, DAO, ENABLED, FILTER, LOG, NEXT_ID,
    PENDING_FILTER_INSTALL, PROTOBUF_REGISTRY, ROLES,
};

pub(crate) const CONTRACT_NAME: &str = "crates.io:dao-rbam";
pub(crate) const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const DEFAULT_LIMIT: u32 = 30;
const DEFAULT_LIMIT_IS_MSG_AUTHORIZED: u32 = 30;
const MAX_LIMIT: u32 = 100;

pub const INSTANTIATE_FILTER_REPLY_ID: u64 = 1;
pub const INSTANTIATE_PROTOBUF_REGISTRY_REPLY_ID: u64 = 2;
pub const PREPARE_PROTOBUF_REGISTRY_REPLY_ID: u64 = 3;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
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
    let owner = msg.owner.unwrap_or_else(|| dao.to_string());
    initialize_owner(deps.storage, deps.api, Some(&owner))?;

    // if protobuf is enabled, we set up the proto registry before
    // setting up the filter. otherwise, just set up the filter.
    let first_init_submsg = match msg.protobuf_registry_code_id {
        Some(protobuf_registry_code_id) => {
            // Save filter code ID so we can instantiate it in the reply handler
            // after the protobuf registry is instantiated.
            PENDING_FILTER_INSTALL.save(
                deps.storage,
                &PendingFilterInstall {
                    filter_code_id: msg.filter_code_id,
                    filter_salt: msg.filter_salt,
                },
            )?;

            submsg_instantiate_registry(
                &env,
                owner,
                protobuf_registry_code_id,
                msg.protobuf_registry_salt,
            )?
        }
        None => submsg_instantiate_filter(&env, owner, msg.filter_code_id, msg.filter_salt, None)?,
    };

    // Initialize enabled state (default to true).
    let enabled = msg.enabled.unwrap_or(true);
    ENABLED.save(deps.storage, &enabled)?;

    // Initialize next ID counter.
    NEXT_ID.save(deps.storage, &1)?;

    let mut response = Response::new()
        .add_submessage(first_init_submsg)
        .add_attribute("action", "instantiate")
        .add_attribute("creator", info.sender.as_str())
        .add_attribute("enabled", enabled.to_string());

    // Create initial roles.
    if let Some(initial_roles) = msg.initial_roles {
        let (role_attrs, role_submsgs) = try_create_initial_roles(deps, initial_roles)?;
        response = response.add_attributes(role_attrs);
        response = response.add_submessages(role_submsgs);
    }

    Ok(response)
}

fn try_create_initial_roles(
    mut deps: DepsMut,
    initial_roles: Vec<InitialRole>,
) -> Result<(Vec<Attribute>, Vec<SubMsg>), ContractError> {
    let mut resp_attributes = vec![];
    let mut resp_messages = vec![];

    // preload the protobuf registry for all potential authorizations
    // and pass it as ref to avoid loading every time
    let protobuf_registry = PROTOBUF_REGISTRY.may_load(deps.storage)?;

    for InitialRole {
        name,
        metadata,
        authorizations,
        assignments,
        enabled,
    } in initial_roles
    {
        let enabled = enabled.unwrap_or(true);
        let role = Role::create(deps.branch(), name, metadata, enabled)?;
        resp_attributes.push(attr("role_id", role.id.to_string()));

        // Create initial authorizations for role.
        if let Some(authorizations) = authorizations {
            for InitialAuthorization {
                name,
                metadata,
                filter,
                enabled,
                skip_prepare,
            } in authorizations
            {
                let enabled = enabled.unwrap_or(true);
                let (authorization, mut protobuf_prepare_messages) = Authorization::create(
                    deps.branch(),
                    &protobuf_registry,
                    role.id,
                    name,
                    metadata,
                    filter,
                    enabled,
                )?;
                resp_attributes.push(attr("authorization_id", authorization.id.to_string()));
                if !skip_prepare.unwrap_or_default() {
                    resp_messages.append(&mut protobuf_prepare_messages);
                }
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

    Ok((resp_attributes, resp_messages))
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
        ExecuteMsg::UpdateDao { dao } => execute_update_dao(deps, info, dao),
        ExecuteMsg::UpdateFilter { filter } => execute_update_filter(deps, info, filter),
        ExecuteMsg::UpdateProtobufRegistry { protobuf_registry } => {
            execute_update_protobuf_registry(deps, info, protobuf_registry)
        }
        ExecuteMsg::UpdateEnabled { enabled } => execute_update_enabled(deps, info, enabled),
        ExecuteMsg::ExecuteProtobufRegistry(msg) => {
            execute_execute_protobuf_registry(deps, info, msg)
        }
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
            skip_prepare,
        } => execute_create_authorization(
            deps,
            info,
            role_id,
            name,
            metadata,
            filter,
            enabled,
            skip_prepare,
        ),
        ExecuteMsg::UpdateAuthorization {
            authorization_id,
            name,
            metadata,
            filter,
            enabled,
            skip_prepare,
        } => execute_update_authorization(
            deps,
            info,
            authorization_id,
            name,
            metadata,
            filter,
            enabled,
            skip_prepare,
        ),
        ExecuteMsg::Assign { assign } => execute_assign(deps, info, assign),
        ExecuteMsg::Revoke { revoke } => execute_revoke(deps, info, revoke),
        ExecuteMsg::ExecuteActions { actions } => execute_execute_actions(deps, env, info, actions),
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

fn execute_update_dao(
    deps: DepsMut,
    info: MessageInfo,
    dao: String,
) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    let dao = deps.api.addr_validate(&dao)?;
    DAO.save(deps.storage, &dao)?;

    Ok(Response::new()
        .add_attribute("action", "update_dao")
        .add_attribute("dao", dao.to_string()))
}

fn execute_update_filter(
    deps: DepsMut,
    info: MessageInfo,
    filter: ModuleUpdate,
) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    let filter_message = filter.update(deps, &FILTER, INSTANTIATE_FILTER_REPLY_ID, info.sender)?;

    Ok(Response::new()
        .add_submessages(filter_message)
        .add_attribute("action", "update_filter"))
}

fn execute_update_protobuf_registry(
    deps: DepsMut,
    info: MessageInfo,
    protobuf_registry: Option<ModuleUpdate>,
) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    let protobuf_registry_message = match protobuf_registry {
        Some(protobuf_registry) => protobuf_registry.update(
            deps,
            &PROTOBUF_REGISTRY,
            INSTANTIATE_PROTOBUF_REGISTRY_REPLY_ID,
            &info.sender,
        )?,
        None => {
            PROTOBUF_REGISTRY.remove(deps.storage);
            vec![]
        }
    };

    Ok(Response::new()
        .add_submessages(protobuf_registry_message)
        .add_attribute("action", "update_protobuf_registry"))
}

fn execute_execute_protobuf_registry(
    deps: DepsMut,
    info: MessageInfo,
    msg: cw_protobuf_registry::msg::ExecuteMsg,
) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    let protobuf_registry = PROTOBUF_REGISTRY
        .load(deps.storage)
        .map_err(|_| ContractError::MissingProtobufRegistry {})?;

    let message = WasmMsg::Execute {
        contract_addr: protobuf_registry.to_string(),
        msg: to_json_binary(&msg)?,
        funds: vec![],
    };

    Ok(Response::new()
        .add_message(message)
        .add_attribute("action", "execute_protobuf_registry"))
}

fn execute_update_enabled(
    deps: DepsMut,
    info: MessageInfo,
    enabled: bool,
) -> Result<Response, ContractError> {
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
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    let enabled = enabled.unwrap_or(true);
    let role = Role::create(deps.branch(), name, metadata, enabled)?;
    // preload the protobuf registry for all potential authorizations
    // and pass it as ref to avoid loading every time
    let protobuf_registry = PROTOBUF_REGISTRY.may_load(deps.storage)?;

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
            skip_prepare,
        } in authorizations
        {
            let enabled = enabled.unwrap_or(true);
            let (authorization, messages) = Authorization::create(
                deps.branch(),
                &protobuf_registry,
                role.id,
                name,
                metadata,
                filter,
                enabled,
            )?;
            response = response.add_attribute("authorization_id", authorization.id.to_string());
            if !skip_prepare.unwrap_or_default() {
                response = response.add_submessages(messages);
            }
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
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    let mut role = Role::load(&deps.as_ref(), role_id)?;

    if let Some(name) = name {
        role.name = name;
    }

    metadata.maybe_update(|metadata| {
        role.metadata = metadata;
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

#[allow(clippy::too_many_arguments)]
fn execute_create_authorization(
    deps: DepsMut,
    info: MessageInfo,
    role_id: u64,
    name: String,
    metadata: Option<String>,
    filter: Option<serde_json::Value>,
    enabled: Option<bool>,
    skip_prepare: Option<bool>,
) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    // Ensure role exists.
    Role::ensure_exists(&deps.as_ref(), role_id)?;

    let enabled = enabled.unwrap_or(true);
    let protobuf_registry = PROTOBUF_REGISTRY.may_load(deps.storage)?;
    let (authorization, protobuf_prepare_messages) = Authorization::create(
        deps,
        &protobuf_registry,
        role_id,
        name,
        metadata,
        filter,
        enabled,
    )?;

    // If skip_prepare is true, don't prepare the protobuf messages.
    let messages = if skip_prepare.unwrap_or_default() {
        vec![]
    } else {
        protobuf_prepare_messages
    };

    Ok(Response::new()
        .add_submessages(messages)
        .add_attribute("action", "create_authorization")
        .add_attribute("role_id", authorization.role_id.to_string())
        .add_attribute("authorization_id", authorization.id.to_string())
        .add_attribute("authorization_name", authorization.name))
}

#[allow(clippy::too_many_arguments)]
fn execute_update_authorization(
    deps: DepsMut,
    info: MessageInfo,
    authorization_id: u64,
    name: Option<String>,
    metadata: OptionalUpdate<String>,
    filter: OptionalUpdate<serde_json::Value>,
    enabled: Option<bool>,
    skip_prepare: Option<bool>,
) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    let mut authorization = Authorization::load(&deps.as_ref(), authorization_id)?;

    if let Some(name) = name {
        authorization.name = name;
    }

    metadata.maybe_update(|value| {
        authorization.metadata = value;
    });

    let protobuf_prepare_messages = filter.maybe_update_result_with_value(
        |value| {
            authorization.filter = value;
            let protobuf_registry = PROTOBUF_REGISTRY.may_load(deps.storage)?;
            authorization.get_protobuf_message_preparation_submsgs(&protobuf_registry)
        },
        vec![],
    )?;

    if let Some(enabled) = enabled {
        authorization.enabled = enabled;
    }

    authorization.save(deps)?;

    // If skip_prepare is true, don't prepare the protobuf messages.
    let messages = if skip_prepare.unwrap_or_default() {
        vec![]
    } else {
        protobuf_prepare_messages
    };

    Ok(Response::new()
        .add_submessages(messages)
        .add_attribute("action", "update_authorization")
        .add_attribute("authorization_id", authorization.id.to_string())
        .add_attribute("authorization_name", authorization.name))
}

fn execute_assign(
    mut deps: DepsMut,
    info: MessageInfo,
    assign: Vec<Assignment>,
) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    if assign.is_empty() {
        return Err(ContractError::NoRoles {});
    }

    let mut role_exists: HashSet<u64> = HashSet::with_capacity(assign.len());

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

    // preload the filter and pass as ref for all potential actions
    // to avoid loading every time
    let filter_contract = FILTER.load(deps.storage)?;

    let (msgs, action_ids): (Vec<_>, Vec<_>) = actions
        .into_iter()
        .map(|action| {
            let action = action.initiate(deps.branch(), &env, &info.sender, &filter_contract)?;
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
        QueryMsg::Info {} => to_json_binary(&query_info(deps)?),
        QueryMsg::Dao {} => to_json_binary(&query_dao(deps)?),
        QueryMsg::Filter {} => to_json_binary(&query_filter(deps)?),
        QueryMsg::ProtobufRegistry {} => to_json_binary(&query_protobuf_registry(deps)?),
        QueryMsg::Enabled {} => to_json_binary(&query_enabled(deps)?),
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
        QueryMsg::Assigned { addr, role_id } => {
            to_json_binary(&query_assigned(deps, addr, role_id)?)
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
        QueryMsg::Authorized {
            addr,
            msg,
            start_after,
            limit,
        } => to_json_binary(&query_authorized(deps, addr, msg, start_after, limit)?),
        QueryMsg::AuthorizedByRole {
            addr,
            role_id,
            msg,
            start_after,
            limit,
        } => to_json_binary(&query_authorized_by_role(
            deps,
            addr,
            role_id,
            msg,
            start_after,
            limit,
        )?),
        QueryMsg::AuthorizedBy {
            addr,
            authorization_id,
            msg,
        } => to_json_binary(&query_authorized_by(deps, addr, authorization_id, msg)?),
        QueryMsg::TestFilter { filter, msg } => {
            to_json_binary(&query_test_filter(deps, filter, msg)?)
        }
    }
}

fn query_info(deps: Deps) -> StdResult<InfoResponse> {
    let info = cw2::get_contract_version(deps.storage)?;
    Ok(InfoResponse { info })
}

fn query_dao(deps: Deps) -> StdResult<DaoResponse> {
    let dao = DAO.load(deps.storage)?;
    Ok(DaoResponse { dao })
}

fn query_filter(deps: Deps) -> StdResult<FilterResponse> {
    let filter = FILTER.load(deps.storage)?;
    Ok(FilterResponse { filter })
}

fn query_protobuf_registry(deps: Deps) -> StdResult<ProtobufRegistryResponse> {
    let protobuf_registry = PROTOBUF_REGISTRY.may_load(deps.storage)?;
    Ok(ProtobufRegistryResponse { protobuf_registry })
}

fn query_enabled(deps: Deps) -> StdResult<EnabledResponse> {
    let enabled = ENABLED.load(deps.storage)?;
    Ok(EnabledResponse { enabled })
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

fn query_assigned(deps: Deps, addr: String, role_id: u64) -> StdResult<AssignedResponse> {
    let addr = deps.api.addr_validate(&addr)?;
    let assigned = Role::is_assigned(&deps, &addr, role_id);
    Ok(AssignedResponse { assigned })
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

fn query_authorized(
    deps: Deps,
    addr: String,
    msg: CosmosMsg,
    start_after: Option<(u64, u64)>,
    limit: Option<u32>,
) -> StdResult<AuthorizedResponse> {
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

    // preload the filter and pass as ref for all potential authorizations
    // to avoid loading every time
    let filter_contract = FILTER.load(deps.storage)?;

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
                return Ok(AuthorizedResponse::Unauthorized {
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
                .allows(&deps, &filter_contract, msg.clone(), true)
                // Should not happen since we ignore filter errors.
                .map_err(|e| StdError::generic_err(e.to_string()))?;

            if allowed {
                return Ok(AuthorizedResponse::Authorized {
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

    Ok(AuthorizedResponse::Unauthorized {
        reason: ContractError::NoMoreAuthorizations {}.to_string(),
        last_checked,
    })
}

fn query_authorized_by_role(
    deps: Deps,
    addr: String,
    role_id: u64,
    msg: CosmosMsg,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<AuthorizedByRoleResponse> {
    let addr = deps.api.addr_validate(&addr)?;
    let mut remaining_to_check = limit.unwrap_or(DEFAULT_LIMIT_IS_MSG_AUTHORIZED) as usize;
    let start = start_after.map(Bound::exclusive);

    // Ensure the role exists.
    let role = match Role::load(&deps, role_id) {
        Ok(role) => role,
        Err(e) => {
            return Ok(AuthorizedByRoleResponse::Unauthorized {
                reason: e.to_string(),
                last_checked: None,
            })
        }
    };

    // Ensure the address is assigned the role.
    let assigned = Role::is_assigned(&deps, &addr, role_id);
    if !assigned {
        return Ok(AuthorizedByRoleResponse::Unauthorized {
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
        return Ok(AuthorizedByRoleResponse::Unauthorized {
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
    let filter_contract = FILTER.load(deps.storage)?;

    for result in authorizations {
        // If we've reached the limit in the past iteration and we have another
        // to check, return error. This ensures we only ever return "limit
        // reached" if we haven't checked all authorizations. If we happen to
        // run out of the limit on the last authorization, and it doesn't match,
        // we don't want to return "limit reached".
        if remaining_to_check == 0 {
            return Ok(AuthorizedByRoleResponse::Unauthorized {
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
            .allows(&deps, &filter_contract, msg.clone(), true)
            // Should not happen since we ignore filter errors.
            .map_err(|e| StdError::generic_err(e.to_string()))?;

        if allowed {
            return Ok(AuthorizedByRoleResponse::Authorized {
                role,
                authorization,
            });
        }
    }

    Ok(AuthorizedByRoleResponse::Unauthorized {
        reason: ContractError::NoMoreAuthorizations {}.to_string(),
        last_checked,
    })
}

fn query_authorized_by(
    deps: Deps,
    addr: String,
    authorization_id: u64,
    msg: CosmosMsg,
) -> StdResult<AuthorizedByResponse> {
    let addr = deps.api.addr_validate(&addr)?;

    // Ensure the role and authorization exist.
    let authorization = match Authorization::load(&deps, authorization_id) {
        Ok(authorization) => authorization,
        Err(e) => {
            return Ok(AuthorizedByResponse::Unauthorized {
                reason: e.to_string(),
            });
        }
    };
    let role = match Role::load(&deps, authorization.role_id) {
        Ok(role) => role,
        Err(e) => {
            return Ok(AuthorizedByResponse::Unauthorized {
                reason: e.to_string(),
            });
        }
    };

    // Ensure address has the role assigned.
    let assigned = Role::is_assigned(&deps, &addr, role.id);
    if !assigned {
        return Ok(AuthorizedByResponse::Unauthorized {
            reason: ContractError::RoleNotAssigned {
                addr: addr.to_string(),
                role_id: role.id,
            }
            .to_string(),
        });
    }

    // Ensure role is enabled.
    if !role.enabled {
        return Ok(AuthorizedByResponse::Unauthorized {
            reason: ContractError::RoleDisabled {}.to_string(),
        });
    }

    // Ensure authorization is enabled.
    if !authorization.enabled {
        return Ok(AuthorizedByResponse::Unauthorized {
            reason: ContractError::AuthorizationDisabled {}.to_string(),
        });
    }

    // Check if the authorization allows the message.
    let filter_contract = FILTER.load(deps.storage)?;
    let allowed = match authorization.allows(&deps, &filter_contract, msg, false) {
        Ok(allowed) => allowed,
        Err(e) => {
            return Ok(AuthorizedByResponse::Unauthorized {
                reason: e.to_string(),
            });
        }
    };

    // should always be true since ignore_filter_error is false. the error above
    // would have returned an error.
    if allowed {
        return Ok(AuthorizedByResponse::Authorized {
            role,
            authorization,
        });
    }

    // should never happen since ignore_filter_error is false. allowed should
    // always be true, otherwise `allows` would have returned an error.
    Ok(AuthorizedByResponse::Unauthorized {
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
    let filter_contract = FILTER.load(deps.storage)?;
    // Test the filter.
    let result = Authorization::filter_allows(&deps, &filter_contract, filter, msg, false);

    // Handle filter errors appropriately.
    let response = result.map_or_else(
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

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        INSTANTIATE_FILTER_REPLY_ID => {
            let res = parse_reply_instantiate_data(msg)?;
            let addr = deps.api.addr_validate(&res.contract_address)?;

            FILTER.save(deps.storage, &addr)?;

            Ok(Response::default().add_attribute("filter", addr))
        }
        INSTANTIATE_PROTOBUF_REGISTRY_REPLY_ID => {
            let res = parse_reply_instantiate_data(msg)?;
            let addr = deps.api.addr_validate(&res.contract_address)?;

            PROTOBUF_REGISTRY.save(deps.storage, &addr)?;

            // If filter code ID is set from instantiate, create it.
            let submsgs = match PENDING_FILTER_INSTALL.may_load(deps.storage)? {
                None => vec![],
                Some(pending_filter_install) => {
                    let owner = cw_ownable::get_ownership(deps.storage)?.owner.unwrap();

                    // Remove the code ID and salt from storage.
                    PENDING_FILTER_INSTALL.remove(deps.storage);

                    let filter_init_msg = submsg_instantiate_filter(
                        &env,
                        owner.to_string(),
                        pending_filter_install.filter_code_id,
                        pending_filter_install.filter_salt,
                        Some(addr.clone()),
                    )?;

                    vec![filter_init_msg]
                }
            };

            Ok(Response::default()
                .add_submessages(submsgs)
                .add_attribute("protobuf_registry", addr))
        }
        PREPARE_PROTOBUF_REGISTRY_REPLY_ID => {
            match msg.result.into_result() {
                Err(reason) => Err(ContractError::ProtobufRegistryPrepareFailed { reason }),
                // Should not happen since we only reply on error.
                Ok(_) => Ok(Response::default()),
            }
        }

        _ => Err(ContractError::UnknownReplyID { id: msg.id }),
    }
}
