use cosmwasm_schema::{cw_serde, QueryResponses};

use cosmwasm_std::{Addr, CosmosMsg};
pub use cw_ownable::Ownership;
use cw_ownable::{cw_ownable_execute, cw_ownable_query};
use dao_interface::helpers::OptionalUpdate;

use crate::action::{Action, ActionToExecute};
use crate::role::{Authorization, Role};

#[cw_serde]
pub struct InstantiateMsg {
    /// The address of the initial owner of the contract. Defaults to the DAO.
    pub owner: Option<String>,
    /// The address of the DAO to execute actions on.
    pub dao: Option<String>,
    /// Whether the RBAM system starts enabled. Defaults to true.
    pub enabled: Option<bool>,
    /// Initial roles to create.
    pub initial_roles: Option<Vec<InitialRole>>,
}

#[cw_serde]
pub struct InitialRole {
    pub name: String,
    pub metadata: Option<String>,
    pub authorizations: Option<Vec<InitialAuthorization>>,
    pub assignments: Option<Vec<String>>,
}

#[cw_serde]
pub struct InitialAuthorization {
    pub name: String,
    pub metadata: Option<String>,
    pub filter: Option<serde_json::Value>,
    pub enabled: Option<bool>,
}

#[cw_serde]
pub struct Assignment {
    pub addr: String,
    pub role_id: u64,
}

#[cw_ownable_execute]
#[cw_serde]
pub enum ExecuteMsg {
    /// Update the DAO to execute actions on. Make sure to add this module to
    /// the DAO's proposal modules list so it is authorized to execute actions.
    UpdateDao {
        dao: String,
    },
    /// Enable or disable the RBAM system globally
    SetEnabled {
        enabled: bool,
    },

    /// Role management
    CreateRole {
        name: String,
        metadata: Option<String>,
        enabled: Option<bool>,
        authorizations: Option<Vec<InitialAuthorization>>,
        assignments: Option<Vec<String>>,
    },
    UpdateRole {
        role_id: u64,
        name: Option<String>,
        metadata: OptionalUpdate<String>,
        enabled: Option<bool>,
    },

    /// Authorization management
    CreateAuthorization {
        role_id: u64,
        name: String,
        metadata: Option<String>,
        filter: Option<serde_json::Value>,
        enabled: Option<bool>,
    },
    UpdateAuthorization {
        authorization_id: u64,
        name: Option<String>,
        metadata: OptionalUpdate<String>,
        filter: OptionalUpdate<serde_json::Value>,
        enabled: Option<bool>,
    },

    /// Assignment management
    Assign {
        assign: Vec<Assignment>,
    },
    Revoke {
        revoke: Vec<Assignment>,
    },

    /// Action execution
    ExecuteActions {
        actions: Vec<ActionToExecute>,
    },
}

#[cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// System queries
    #[returns(DaoResponse)]
    Dao {},
    #[returns(IsEnabledResponse)]
    IsEnabled {},

    /// Role queries
    #[returns(RoleResponse)]
    Role { id: u64 },
    #[returns(ListRolesResponse)]
    ListRoles {
        start_after: Option<u64>,
        limit: Option<u32>,
    },

    /// Authorization queries
    #[returns(AuthorizationResponse)]
    Authorization { id: u64 },
    #[returns(ListAuthorizationsResponse)]
    ListAuthorizations {
        start_after: Option<u64>,
        limit: Option<u32>,
    },
    #[returns(ListAuthorizationsResponse)]
    ListAuthorizationsByRole {
        role_id: u64,
        start_after: Option<u64>,
        limit: Option<u32>,
    },

    /// Assignment queries
    #[returns(IsAssignedRoleResponse)]
    IsAssignedRole { addr: String, role_id: u64 },
    #[returns(ListAssignmentsResponse)]
    ListAssignments {
        /// The (addr, role_id) to start after. If not provided, the query will
        /// start from the beginning.
        start_after: Option<(String, u64)>,
        /// The maximum number of assignments to return. Defaults to 10, max is
        /// 100.
        limit: Option<u32>,
    },
    #[returns(ListAddressesWithRoleResponse)]
    ListAddressesWithRole {
        /// The role to list assigned addresses for.
        role_id: u64,
        /// The address to start after. If not provided, the query will start
        /// from the beginning.
        start_after: Option<String>,
        /// The maximum number of addresses to return. Defaults to 10, max is
        /// 100.
        limit: Option<u32>,
    },
    #[returns(ListRolesForAddressResponse)]
    ListRolesForAddress {
        /// The address to list assigned roles for.
        addr: String,
        /// The role ID to start after. If not provided, the query will start
        /// from the beginning.
        start_after: Option<u64>,
        /// The maximum number of roles to return. Defaults to 10, max is 100.
        limit: Option<u32>,
    },

    /// Action/Log queries
    #[returns(ActionResponse)]
    Action { addr: String, id: u64 },
    #[returns(ListActionsResponse)]
    ListActions {
        /// The action ID to start after. If not provided, the query will start
        /// from the beginning.
        start_after: Option<u64>,
        /// The maximum number of actions to return. Defaults to 10, max is 100.
        limit: Option<u32>,
        /// Whether to reverse the order of the results. Defaults to false. If
        /// reversed, start_after is actually the exclusive upper bound (i.e. it
        /// becomes start_before).
        reverse: Option<bool>,
    },
    #[returns(ListActionsResponse)]
    ListActionsByRole {
        /// The role to list actions for.
        role_id: u64,
        /// The (addr, action_id) to start after. If not provided, the query
        /// will start from the beginning.
        start_after: Option<(String, u64)>,
        /// The maximum number of actions to return. Defaults to 10, max is 100.
        limit: Option<u32>,
        /// Whether to reverse the order of the results. Defaults to false. If
        /// reversed, start_after is actually the exclusive upper bound (i.e. it
        /// becomes start_before).
        reverse: Option<bool>,
    },
    #[returns(ListActionsResponse)]
    ListActionsByAuthorization {
        /// The authorization to list actions for.
        authorization_id: u64,
        /// The (addr, action_id) to start after. If not provided, the query
        /// will start from the beginning.
        start_after: Option<(String, u64)>,
        /// The maximum number of actions to return. Defaults to 10, max is 100.
        limit: Option<u32>,
        /// Whether to reverse the order of the results. Defaults to false. If
        /// reversed, start_after is actually the exclusive upper bound (i.e. it
        /// becomes start_before).
        reverse: Option<bool>,
    },
    #[returns(ListActionsResponse)]
    ListActionsByAddress {
        /// The address to list actions for.
        addr: String,
        /// The action ID to start after. If not provided, the query will start
        /// from the beginning.
        start_after: Option<u64>,
        /// The maximum number of actions to return. Defaults to 10, max is 100.
        limit: Option<u32>,
        /// Whether to reverse the order of the results. Defaults to false. If
        /// reversed, start_after is actually the exclusive upper bound (i.e. it
        /// becomes start_before).
        reverse: Option<bool>,
    },

    /// Authorization validation queries
    #[returns(IsMsgAuthorizedResponse)]
    IsMsgAuthorized {
        /// The address to check authorization for.
        addr: String,
        /// The message to check authorization for.
        msg: CosmosMsg,
        /// The (role_id, authorization_id) to start after. If not provided, the
        /// query will start from the beginning.
        start_after: Option<(u64, u64)>,
        /// The maximum number of authorizations to check. Defaults to 30.
        limit: Option<u32>,
    },
    #[returns(IsMsgAuthorizedByRoleResponse)]
    IsMsgAuthorizedByRole {
        /// The address to check authorization for.
        addr: String,
        /// The role to check authorization for.
        role_id: u64,
        /// The message to check authorization for.
        msg: CosmosMsg,
        /// The authorization_id to start after. If not provided, the query will
        /// start from the beginning.
        start_after: Option<u64>,
        /// The maximum number of authorizations to check. Defaults to 30.
        limit: Option<u32>,
    },
    #[returns(IsMsgAuthorizedByResponse)]
    IsMsgAuthorizedBy {
        /// The address to check authorization for.
        addr: String,
        /// The role to check authorization for.
        role_id: u64,
        /// The authorization to check authorization for.
        authorization_id: u64,
        /// The message to check authorization for.
        msg: CosmosMsg,
    },

    // Helpers
    #[returns(TestFilterResponse)]
    TestFilter {
        filter: serde_json::Value,
        msg: CosmosMsg,
    },
}

#[cw_serde]
pub struct MigrateMsg {}

// Response types
#[cw_serde]
pub struct DaoResponse {
    pub dao: Addr,
}

#[cw_serde]
pub struct IsEnabledResponse {
    pub enabled: bool,
}

#[cw_serde]
pub struct RoleResponse {
    pub role: Role,
}

#[cw_serde]
pub struct ListRolesResponse {
    pub roles: Vec<Role>,
}

#[cw_serde]
pub struct AuthorizationResponse {
    pub authorization: Authorization,
}

#[cw_serde]
pub struct ListAuthorizationsResponse {
    pub authorizations: Vec<Authorization>,
}

#[cw_serde]
pub struct IsAssignedRoleResponse {
    pub assigned: bool,
}

#[cw_serde]
pub struct ListAssignmentsResponse {
    pub assignments: Vec<Assignment>,
}

#[cw_serde]
pub struct ListAddressesWithRoleResponse {
    pub addresses: Vec<Addr>,
}

#[cw_serde]
pub struct ListRolesForAddressResponse {
    pub role_ids: Vec<u64>,
}

#[cw_serde]
pub struct ActionResponse {
    pub action: Action,
}

#[cw_serde]
pub struct ListActionsResponse {
    pub actions: Vec<Action>,
}

#[cw_serde]
pub enum IsMsgAuthorizedResponse {
    Authorized {
        /// The role that matched.
        role: Role,
        /// The authorization that matched.
        authorization: Authorization,
    },
    Unauthorized {
        /// The reason for the authorization failure.
        reason: String,
        /// The last role and authorization that was checked, if any existed. If
        /// this is set, use it as the start_after for the next query.
        last_checked: Option<(u64, u64)>,
    },
}

#[cw_serde]
pub enum IsMsgAuthorizedByRoleResponse {
    Authorized {
        role: Role,
        authorization: Authorization,
    },
    Unauthorized {
        /// The reason for the authorization failure.
        reason: String,
        /// The last authorization that was checked, if any existed. If this is
        /// set, use it as the start_after for the next query.
        last_checked: Option<u64>,
    },
}

#[cw_serde]
pub enum IsMsgAuthorizedByResponse {
    Authorized {
        role: Role,
        authorization: Authorization,
    },
    Unauthorized {
        /// The reason for the authorization failure.
        reason: String,
    },
}

#[cw_serde]
pub enum TestFilterResponse {
    Pass {},
    Fail {
        /// The reason for the filter failing.
        reason: String,
    },
}
