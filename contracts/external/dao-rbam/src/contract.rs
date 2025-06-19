use std::collections::HashSet;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Order, Response, StdError, StdResult,
};
use cw_storage_plus::Bound;

use cw2::set_contract_version;
use cw_ownable::initialize_owner;
use cw_utils::nonpayable;
use dao_interface::helpers::OptionalUpdate;

use crate::action::{Action, ActionToExecute};
use crate::error::ContractError;
use crate::helpers::ensure_enabled;
use crate::msg::{
    ActionResponse, AssignmentPair, AuthorizationResponse, ExecuteMsg, InitialAuthorization,
    InitialRole, InstantiateMsg, IsAssignedRoleResponse, IsEnabledResponse,
    IsMsgAuthorizedResponse, ListActionsResponse, ListAddressesWithRoleResponse,
    ListAssignmentsResponse, ListAuthorizationsResponse, ListRolesForAddressResponse,
    ListRolesResponse, MigrateMsg, QueryMsg, RoleResponse,
};
use crate::role::{Authorization, Role};
use crate::state::{ASSIGNMENTS, AUTHORIZATIONS, ENABLED, LOG, NEXT_ID, ROLES};

pub(crate) const CONTRACT_NAME: &str = "crates.io:dao-rbam";
pub(crate) const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const DEFAULT_LIMIT: u32 = 10;
const MAX_LIMIT: u32 = 100;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let owner = msg.owner.map_or_else(
        || Ok(info.sender.clone()),
        |owner| deps.api.addr_validate(&owner),
    )?;
    initialize_owner(deps.storage, deps.api, Some(owner.as_str()))?;

    // Initialize enabled state (default to true).
    let enabled = msg.enabled.unwrap_or(true);
    ENABLED.save(deps.storage, &enabled)?;

    // Initialize next ID counter.
    NEXT_ID.save(deps.storage, &1)?;

    let mut response = Response::new()
        .add_attribute("method", "instantiate")
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
        ExecuteMsg::SetEnabled { enabled } => execute_set_enabled(deps, info, enabled),
        ExecuteMsg::CreateRole {
            name,
            metadata,
            enabled,
        } => execute_create_role(deps, info, name, metadata, enabled),
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
        ExecuteMsg::ExecuteActions { actions } => execute_actions(deps, env, info, actions),
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
        .add_attribute("method", "update_ownership")
        .add_attributes(ownership.into_attributes()))
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
        .add_attribute("method", "set_enabled")
        .add_attribute("enabled", enabled.to_string()))
}

fn execute_create_role(
    deps: DepsMut,
    info: MessageInfo,
    name: String,
    metadata: Option<String>,
    enabled: Option<bool>,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    let enabled = enabled.unwrap_or(true);
    let role = Role::create(deps, name, metadata, enabled)?;

    Ok(Response::new()
        .add_attribute("method", "create_role")
        .add_attribute("role_id", role.id.to_string())
        .add_attribute("role_name", role.name))
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
        .add_attribute("method", "update_role")
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
        .add_attribute("method", "create_authorization")
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

    filter.maybe_update(|value| {
        authorization.filter = value;
    });

    if let Some(enabled) = enabled {
        authorization.enabled = enabled;
    }

    authorization.save(deps)?;

    Ok(Response::new()
        .add_attribute("method", "update_authorization")
        .add_attribute("authorization_id", authorization.id.to_string())
        .add_attribute("authorization_name", authorization.name))
}

fn execute_assign(
    mut deps: DepsMut,
    info: MessageInfo,
    assign: Vec<AssignmentPair>,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    let mut role_exists: HashSet<u64> = HashSet::new();

    for AssignmentPair { addr, role_id } in assign {
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

    Ok(Response::new().add_attribute("method", "assign"))
}

fn execute_revoke(
    mut deps: DepsMut,
    info: MessageInfo,
    revoke: Vec<AssignmentPair>,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    for AssignmentPair { addr, role_id } in revoke {
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

    Ok(Response::new().add_attribute("method", "revoke"))
}

fn execute_actions(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    actions: Vec<ActionToExecute>,
) -> Result<Response, ContractError> {
    ensure_enabled(deps.branch())?;

    let (msgs, action_ids): (Vec<_>, Vec<_>) = actions
        .into_iter()
        .map(|action| {
            let action = action.initiate(deps.branch(), &env, &info.sender)?;
            Ok((action.msg, action.id.to_string()))
        })
        .collect::<Result<Vec<_>, ContractError>>()?
        .into_iter()
        .unzip();

    Ok(Response::new()
        .add_messages(msgs)
        .add_attribute("method", "execute_actions")
        .add_attribute("action_ids", action_ids.join(",")))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Ownership {} => to_json_binary(&cw_ownable::get_ownership(deps.storage)?),
        QueryMsg::IsEnabled {} => to_json_binary(&query_is_enabled(deps)?),
        QueryMsg::GetRole { id } => to_json_binary(&query_get_role(deps, id)?),
        QueryMsg::ListRoles { start_after, limit } => {
            to_json_binary(&query_list_roles(deps, start_after, limit)?)
        }
        QueryMsg::GetAuthorization { id } => to_json_binary(&query_get_authorization(deps, id)?),
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
        QueryMsg::GetAction { addr, id } => to_json_binary(&query_get_action(deps, addr, id)?),
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
    }
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
    let assigned = ASSIGNMENTS.has(deps.storage, (addr, role_id));
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
    let action = LOG.load(deps.storage, (addr, action_id))?;
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
    msg: cosmwasm_std::CosmosMsg,
    start_after: Option<(u64, u64)>,
    limit: Option<u32>,
) -> StdResult<IsMsgAuthorizedResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let addr = deps.api.addr_validate(&addr)?;

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

        let role = ROLES.load(deps.storage, role_id)?;
        // Skip if the role is disabled.
        if !role.enabled {
            continue;
        }

        // Get authorizations for this role.
        let authorizations = AUTHORIZATIONS
            .idx
            .role_id
            .prefix(role.id)
            .range(deps.storage, start_auth, None, Order::Ascending)
            .take(limit)
            .map(|item| item.map(|(_, authorization)| authorization))
            .collect::<StdResult<Vec<_>>>()?;

        for authorization in authorizations {
            last_checked = Some((role.id, authorization.id));

            // Skip if the authorization is disabled.
            if !authorization.enabled {
                continue;
            }

            // Check if the authorization allows the message.
            let allowed = authorization
                .allows(&msg, true)
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

    Ok(IsMsgAuthorizedResponse::Unauthorized { last_checked })
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    // Set contract to version to latest
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}
