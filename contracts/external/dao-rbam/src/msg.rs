use cosmwasm_schema::{cw_serde, QueryResponses};

use cosmwasm_std::{Addr, Binary, CosmosMsg};
use cw_ownable::{cw_ownable_execute, cw_ownable_query};
use dao_interface::helpers::OptionalUpdate;
use dao_interface::proposal::InfoResponse;
use dao_interface::state::ModuleUpdate;

use crate::action::{Action, ActionToExecute};
use crate::role::{Authorization, Role};

pub use cw_ownable::Ownership;

#[cw_serde]
pub struct InstantiateMsg {
    /// The address of the initial owner of the contract. Defaults to the DAO.
    pub owner: Option<String>,
    /// The address of the DAO to execute actions on.
    pub dao: Option<String>,
    /// The filter code ID to instantiate.
    pub filter_code_id: u64,
    /// The filter salt to use for instantiate2.
    pub filter_salt: Option<Binary>,
    /// The protobuf registry code ID to instantiate.
    pub protobuf_registry_code_id: Option<u64>,
    /// The protobuf registry salt to use for instantiate.
    pub protobuf_registry_salt: Option<Binary>,
    /// Whether the RBAM system starts enabled. Defaults to true.
    pub enabled: Option<bool>,
    /// Initial roles to create.
    pub initial_roles: Option<Vec<InitialRole>>,
}

#[cw_serde]
pub struct InitialRole {
    /// The name for the role.
    pub name: String,
    /// Optionally set metadata for the role.
    pub metadata: Option<String>,
    /// Optionally set whether the role is enabled.
    pub enabled: Option<bool>,
    /// Optionally create authorizations for the role.
    pub authorizations: Option<Vec<InitialAuthorization>>,
    /// Optionally assign the role to addresses immediately.
    pub assignments: Option<Vec<String>>,
}

#[cw_serde]
pub struct InitialAuthorization {
    /// The name for the authorization.
    pub name: String,
    /// Optionally set metadata for the authorization.
    pub metadata: Option<String>,
    /// Optionally set the filter for the authorization.
    pub filter: Option<serde_json::Value>,
    /// Optionally set whether the authorization is enabled.
    pub enabled: Option<bool>,
    /// Optionally skip preparing the protobuf messages, if any. Defaults to
    /// false, meaning it will prepare the protobuf messages.
    pub skip_prepare: Option<bool>,
}

#[cw_serde]
pub struct Assignment {
    /// The address to assign the role to.
    pub addr: String,
    /// The role ID to assign.
    pub role_id: u64,
}

#[cw_ownable_execute]
#[cw_serde]
pub enum ExecuteMsg {
    // System management
    /// Update the DAO to execute actions on. Make sure to add this module to
    /// the DAO's proposal modules list so it is authorized to execute actions.
    UpdateDao {
        /// The address of the DAO to execute actions on.
        dao: String,
    },
    /// Update the filter contract.
    UpdateFilter { filter: ModuleUpdate },
    /// Update the protobuf registry contract.
    UpdateProtobufRegistry {
        protobuf_registry: Option<ModuleUpdate>,
    },
    /// Enable or disable the RBAM system globally.
    UpdateEnabled {
        /// Whether actions are allowed to be executed.
        enabled: bool,
    },
    /// Execute actions on the protobuf registry contract (assuming this
    /// contract is the owner).
    ExecuteProtobufRegistry(cw_protobuf_registry::msg::ExecuteMsg),

    // Role management
    /// Create a new role.
    CreateRole {
        /// The name for the role.
        name: String,
        /// Optionally set metadata for the role.
        metadata: Option<String>,
        /// Optionally set whether the role is enabled.
        enabled: Option<bool>,
        /// Optionally create authorizations for the role.
        authorizations: Option<Vec<InitialAuthorization>>,
        /// Optionally assign the role to addresses immediately.
        assignments: Option<Vec<String>>,
    },
    /// Update a role.
    UpdateRole {
        /// The role ID to update.
        role_id: u64,
        /// Optionally update the name for the role.
        name: Option<String>,
        /// Optionally update the metadata for the role.
        metadata: OptionalUpdate<String>,
        /// Optionally update whether the role is enabled.
        enabled: Option<bool>,
    },

    // Authorization management
    /// Create a new authorization.
    CreateAuthorization {
        /// The role ID to create the authorization for.
        role_id: u64,
        /// The name for the authorization.
        name: String,
        /// Optionally set metadata for the authorization.
        metadata: Option<String>,
        /// Optionally set the filter for the authorization.
        filter: Option<serde_json::Value>,
        /// Optionally set whether the authorization is enabled.
        enabled: Option<bool>,
        /// Optionally skip preparing the protobuf messages, if any. Defaults to
        /// false, meaning it will prepare the protobuf messages.
        skip_prepare: Option<bool>,
    },
    /// Update an authorization.
    UpdateAuthorization {
        /// The authorization ID to update.
        authorization_id: u64,
        /// Optionally update the name for the authorization.
        name: Option<String>,
        /// Optionally update the metadata for the authorization.
        metadata: OptionalUpdate<String>,
        /// Optionally update the filter for the authorization.
        filter: OptionalUpdate<serde_json::Value>,
        /// Optionally update whether the authorization is enabled.
        enabled: Option<bool>,
        /// Optionally skip preparing the protobuf messages, if they changed.
        /// Defaults to false, meaning it will prepare the protobuf messages.
        skip_prepare: Option<bool>,
    },

    // Assignment management
    /// Assign roles to addresses.
    Assign {
        /// The assignments to create.
        assign: Vec<Assignment>,
    },
    /// Revoke roles from addresses.
    Revoke {
        /// The assignments to revoke.
        revoke: Vec<Assignment>,
    },

    // Action execution
    /// Execute actions on behalf of the DAO.
    ExecuteActions {
        /// The actions to execute.
        actions: Vec<ActionToExecute>,
    },
}

#[cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    // System queries
    #[returns(InfoResponse)]
    Info {},
    #[returns(DaoResponse)]
    Dao {},
    #[returns(FilterResponse)]
    Filter {},
    #[returns(ProtobufRegistryResponse)]
    ProtobufRegistry {},
    #[returns(EnabledResponse)]
    Enabled {},

    // Role queries
    #[returns(RoleResponse)]
    Role {
        /// The role ID.
        id: u64,
    },
    #[returns(ListRolesResponse)]
    ListRoles {
        /// The role ID to start after. If not provided, the query will start
        /// from the beginning.
        start_after: Option<u64>,
        /// The maximum number of roles to return. Defaults to 10, max is 100.
        limit: Option<u32>,
    },

    // Authorization queries
    #[returns(AuthorizationResponse)]
    Authorization {
        /// The authorization ID.
        id: u64,
    },
    #[returns(ListAuthorizationsResponse)]
    ListAuthorizations {
        /// The authorization ID to start after. If not provided, the query will
        /// start from the beginning.
        start_after: Option<u64>,
        /// The maximum number of authorizations to return. Defaults to 10, max
        /// is 100.
        limit: Option<u32>,
    },
    #[returns(ListAuthorizationsResponse)]
    ListAuthorizationsByRole {
        /// The role ID to list authorizations for.
        role_id: u64,
        /// The authorization ID to start after. If not provided, the query will
        /// start from the beginning.
        start_after: Option<u64>,
        /// The maximum number of authorizations to return. Defaults to 10, max
        /// is 100.
        limit: Option<u32>,
    },

    // Assignment queries
    #[returns(AssignedResponse)]
    Assigned {
        /// The address to check assignment for.
        addr: String,
        /// The role ID to check assignment for.
        role_id: u64,
    },
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

    // Action/Log queries
    #[returns(ActionResponse)]
    Action {
        /// The action ID.
        id: u64,
    },
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
        /// The action_id to start after. If not provided, the query will start
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
    ListActionsByAuthorization {
        /// The authorization to list actions for.
        authorization_id: u64,
        /// The action_id to start after. If not provided, the query will start
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

    // Authorization validation queries
    #[returns(AuthorizedResponse)]
    Authorized {
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
    #[returns(AuthorizedByRoleResponse)]
    AuthorizedByRole {
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
    #[returns(AuthorizedByResponse)]
    AuthorizedBy {
        /// The address to check authorization for.
        addr: String,
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
pub struct FilterResponse {
    pub filter: Addr,
}

#[cw_serde]
pub struct ProtobufRegistryResponse {
    /// The address of the protobuf registry, if set.
    pub protobuf_registry: Option<Addr>,
}

#[cw_serde]
pub struct EnabledResponse {
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
pub struct AssignedResponse {
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
pub enum AuthorizedResponse {
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
pub enum AuthorizedByRoleResponse {
    Authorized {
        /// The role that matched.
        role: Role,
        /// The authorization that matched.
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
pub enum AuthorizedByResponse {
    Authorized {
        /// The role that matched.
        role: Role,
        /// The authorization that matched.
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
