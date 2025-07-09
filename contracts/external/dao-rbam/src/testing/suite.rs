use cosmwasm_std::{to_json_binary, Addr, CosmosMsg, StdError, Timestamp};
use cw4::Member;
use cw_ownable::Action;
use dao_interface::{helpers::OptionalUpdate, proposal::InfoResponse, state::ModuleUpdate};
use dao_rbam::ContractError;
use dao_testing::{DaoTestingSuite, DaoTestingSuiteBase, ADDR0, ADDR1, ADDR2};

use crate::{
    action::ActionToExecute,
    msg::{
        ActionResponse, AssignedResponse, Assignment, AuthorizationResponse, AuthorizedByResponse,
        AuthorizedByRoleResponse, AuthorizedResponse, DaoResponse, EnabledResponse, ExecuteMsg,
        FilterResponse, InitialAuthorization, InitialRole, InstantiateMsg, ListActionsResponse,
        ListAddressesWithRoleResponse, ListAssignmentsResponse, ListAuthorizationsResponse,
        ListRolesForAddressResponse, ListRolesResponse, ProtobufRegistryResponse, QueryMsg,
        RoleResponse, TestFilterResponse,
    },
    role::Role,
};

pub struct SuiteBuilder {
    pub initial_roles: Vec<InitialRole>,
    pub cw4_members: Vec<Member>,
}

impl SuiteBuilder {
    pub fn base() -> Self {
        Self {
            initial_roles: vec![],
            cw4_members: vec![
                Member {
                    addr: ADDR0.to_string(),
                    weight: 1,
                },
                Member {
                    addr: ADDR1.to_string(),
                    weight: 2,
                },
                Member {
                    addr: ADDR2.to_string(),
                    weight: 1,
                },
            ],
        }
    }

    pub fn with_initial_role(mut self, role: InitialRole) -> Self {
        self.initial_roles.push(role);
        self
    }

    pub fn build(self) -> Suite {
        let mut base = DaoTestingSuiteBase::base();
        let dao = base.cw4().with_members(self.cw4_members).dao();

        let crate_root = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
        let google_string_bool_fds =
            std::fs::read(std::path::Path::new(&crate_root).join("proto/string_bool_value.pb"))
                .unwrap();
        let regen_ecocredit_basket_fds =
            std::fs::read(std::path::Path::new(&crate_root).join("proto/regen_ecocredit.pb"))
                .unwrap();

        let mut suite = Suite {
            base,
            core_addr: dao.core_addr.clone(),
            rbam_addr: Addr::unchecked(""),
            filter_addr: Addr::unchecked(""),
            protobuf_registry_addr: Addr::unchecked(""),

            google_string_bool_fds,
            regen_ecocredit_basket_fds,
        };

        // start at 0 height and time
        suite.base.app.update_block(|b| {
            b.height = 0;
            b.time = Timestamp::from_seconds(0);
        });

        // initialize the contract
        let response = suite.base.execute_smart_ok(
            &suite.core_addr,
            &suite.core_addr,
            &dao_interface::msg::ExecuteMsg::UpdateProposalModules {
                to_add: vec![dao_interface::state::ModuleInstantiateInfo {
                    code_id: suite.base.rbam_id,
                    msg: to_json_binary(&InstantiateMsg {
                        owner: None,
                        dao: None,
                        filter_code_id: suite.base.filter_id,
                        filter_salt: None,
                        protobuf_registry_code_id: Some(suite.base.protobuf_registry_id),
                        protobuf_registry_salt: None,
                        enabled: None,
                        initial_roles: if self.initial_roles.is_empty() {
                            None
                        } else {
                            Some(self.initial_roles)
                        },
                    })
                    .unwrap(),
                    admin: Some(dao_interface::state::Admin::CoreModule {}),
                    funds: None,
                    label: "rbam".to_string(),
                    salt: None,
                }],
                to_disable: vec![],
            },
            &[],
        );

        suite.rbam_addr = Addr::unchecked(
            response
                .events
                .iter()
                .find(|e| e.ty == "instantiate")
                .unwrap()
                .attributes
                .iter()
                .find(|a| a.key == "_contract_address")
                .unwrap()
                .value
                .clone(),
        );

        suite.filter_addr = suite.get_filter();
        suite.protobuf_registry_addr = suite
            .base
            .app
            .wrap()
            .query_wasm_smart::<cw_filter::msg::ProtobufRegistryResponse>(
                suite.filter_addr.clone(),
                &cw_filter::msg::QueryMsg::ProtobufRegistry {},
            )
            .unwrap()
            .protobuf_registry
            .unwrap();

        suite
    }
}

pub struct Suite {
    pub base: DaoTestingSuiteBase,
    pub core_addr: Addr,
    pub rbam_addr: Addr,
    pub filter_addr: Addr,
    pub protobuf_registry_addr: Addr,

    // file descriptor sets
    pub google_string_bool_fds: Vec<u8>,
    pub regen_ecocredit_basket_fds: Vec<u8>,
}

// SUITE QUERIES
impl Suite {
    pub fn get_ownership(&mut self) -> cw_ownable::Ownership<Addr> {
        self.base
            .app
            .wrap()
            .query_wasm_smart(self.rbam_addr.clone(), &QueryMsg::Ownership {})
            .unwrap()
    }

    pub fn get_info(&mut self) -> InfoResponse {
        self.base
            .app
            .wrap()
            .query_wasm_smart(self.rbam_addr.clone(), &QueryMsg::Info {})
            .unwrap()
    }

    pub fn get_dao(&mut self) -> Addr {
        self.base
            .app
            .wrap()
            .query_wasm_smart::<DaoResponse>(&self.rbam_addr, &QueryMsg::Dao {})
            .unwrap()
            .dao
    }

    pub fn get_filter(&mut self) -> Addr {
        self.base
            .app
            .wrap()
            .query_wasm_smart::<FilterResponse>(self.rbam_addr.clone(), &QueryMsg::Filter {})
            .unwrap()
            .filter
    }

    pub fn get_protobuf_registry(&mut self) -> Option<Addr> {
        self.base
            .app
            .wrap()
            .query_wasm_smart::<ProtobufRegistryResponse>(
                self.rbam_addr.clone(),
                &QueryMsg::ProtobufRegistry {},
            )
            .unwrap()
            .protobuf_registry
    }

    pub fn get_enabled(&mut self) -> bool {
        self.base
            .app
            .wrap()
            .query_wasm_smart::<EnabledResponse>(self.rbam_addr.clone(), &QueryMsg::Enabled {})
            .unwrap()
            .enabled
    }

    pub fn get_role(&mut self, id: u64) -> Role {
        self.base
            .app
            .wrap()
            .query_wasm_smart::<RoleResponse>(self.rbam_addr.clone(), &QueryMsg::Role { id })
            .unwrap()
            .role
    }

    pub fn list_roles(
        &mut self,
        start_after: Option<u64>,
        limit: Option<u32>,
    ) -> ListRolesResponse {
        self.base
            .app
            .wrap()
            .query_wasm_smart(
                self.rbam_addr.clone(),
                &QueryMsg::ListRoles { start_after, limit },
            )
            .unwrap()
    }

    pub fn get_authorization(&mut self, id: u64) -> AuthorizationResponse {
        self.base
            .app
            .wrap()
            .query_wasm_smart(self.rbam_addr.clone(), &QueryMsg::Authorization { id })
            .unwrap()
    }

    pub fn list_authorizations(
        &mut self,
        start_after: Option<u64>,
        limit: Option<u32>,
    ) -> ListAuthorizationsResponse {
        self.base
            .app
            .wrap()
            .query_wasm_smart(
                self.rbam_addr.clone(),
                &QueryMsg::ListAuthorizations { start_after, limit },
            )
            .unwrap()
    }

    pub fn list_authorizations_by_role(
        &mut self,
        role_id: u64,
        start_after: Option<u64>,
        limit: Option<u32>,
    ) -> ListAuthorizationsResponse {
        self.base
            .app
            .wrap()
            .query_wasm_smart(
                self.rbam_addr.clone(),
                &QueryMsg::ListAuthorizationsByRole {
                    role_id,
                    start_after,
                    limit,
                },
            )
            .unwrap()
    }

    pub fn assigned(&mut self, addr: String, role_id: u64) -> AssignedResponse {
        self.base
            .app
            .wrap()
            .query_wasm_smart(
                self.rbam_addr.clone(),
                &QueryMsg::Assigned { addr, role_id },
            )
            .unwrap()
    }

    pub fn list_assignments(
        &mut self,
        start_after: Option<(String, u64)>,
        limit: Option<u32>,
    ) -> ListAssignmentsResponse {
        self.base
            .app
            .wrap()
            .query_wasm_smart(
                self.rbam_addr.clone(),
                &QueryMsg::ListAssignments { start_after, limit },
            )
            .unwrap()
    }

    pub fn list_addresses_with_role(
        &mut self,
        role_id: u64,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> ListAddressesWithRoleResponse {
        self.base
            .app
            .wrap()
            .query_wasm_smart(
                self.rbam_addr.clone(),
                &QueryMsg::ListAddressesWithRole {
                    role_id,
                    start_after,
                    limit,
                },
            )
            .unwrap()
    }

    pub fn list_roles_for_address(
        &mut self,
        addr: String,
        start_after: Option<u64>,
        limit: Option<u32>,
    ) -> ListRolesForAddressResponse {
        self.base
            .app
            .wrap()
            .query_wasm_smart(
                self.rbam_addr.clone(),
                &QueryMsg::ListRolesForAddress {
                    addr,
                    start_after,
                    limit,
                },
            )
            .unwrap()
    }

    pub fn get_action(&mut self, addr: String, id: u64) -> ActionResponse {
        self.base
            .app
            .wrap()
            .query_wasm_smart(self.rbam_addr.clone(), &QueryMsg::Action { addr, id })
            .unwrap()
    }

    pub fn get_action_err(&mut self, addr: String, id: u64) -> StdError {
        self.base
            .app
            .wrap()
            .query_wasm_smart::<ActionResponse>(
                self.rbam_addr.clone(),
                &QueryMsg::Action { addr, id },
            )
            .unwrap_err()
    }

    pub fn list_actions(
        &mut self,
        start_after: Option<u64>,
        limit: Option<u32>,
        reverse: Option<bool>,
    ) -> ListActionsResponse {
        self.base
            .app
            .wrap()
            .query_wasm_smart(
                self.rbam_addr.clone(),
                &QueryMsg::ListActions {
                    start_after,
                    limit,
                    reverse,
                },
            )
            .unwrap()
    }

    pub fn list_actions_by_role(
        &mut self,
        role_id: u64,
        start_after: Option<(String, u64)>,
        limit: Option<u32>,
        reverse: Option<bool>,
    ) -> ListActionsResponse {
        self.base
            .app
            .wrap()
            .query_wasm_smart(
                self.rbam_addr.clone(),
                &QueryMsg::ListActionsByRole {
                    role_id,
                    start_after,
                    limit,
                    reverse,
                },
            )
            .unwrap()
    }

    pub fn list_actions_by_authorization(
        &mut self,
        authorization_id: u64,
        start_after: Option<(String, u64)>,
        limit: Option<u32>,
        reverse: Option<bool>,
    ) -> ListActionsResponse {
        self.base
            .app
            .wrap()
            .query_wasm_smart(
                self.rbam_addr.clone(),
                &QueryMsg::ListActionsByAuthorization {
                    authorization_id,
                    start_after,
                    limit,
                    reverse,
                },
            )
            .unwrap()
    }

    pub fn list_actions_by_address(
        &mut self,
        addr: String,
        start_after: Option<u64>,
        limit: Option<u32>,
        reverse: Option<bool>,
    ) -> ListActionsResponse {
        self.base
            .app
            .wrap()
            .query_wasm_smart(
                self.rbam_addr.clone(),
                &QueryMsg::ListActionsByAddress {
                    addr,
                    start_after,
                    limit,
                    reverse,
                },
            )
            .unwrap()
    }

    pub fn authorized(
        &mut self,
        addr: String,
        msg: &CosmosMsg,
        start_after: Option<(u64, u64)>,
        limit: Option<u32>,
    ) -> AuthorizedResponse {
        self.base
            .app
            .wrap()
            .query_wasm_smart(
                &self.rbam_addr,
                &QueryMsg::Authorized {
                    addr,
                    msg: msg.clone(),
                    start_after,
                    limit,
                },
            )
            .unwrap()
    }

    pub fn authorized_by_role(
        &mut self,
        addr: String,
        role_id: u64,
        msg: &CosmosMsg,
        start_after: Option<u64>,
        limit: Option<u32>,
    ) -> AuthorizedByRoleResponse {
        self.base
            .app
            .wrap()
            .query_wasm_smart(
                &self.rbam_addr,
                &QueryMsg::AuthorizedByRole {
                    addr,
                    role_id,
                    msg: msg.clone(),
                    start_after,
                    limit,
                },
            )
            .unwrap()
    }

    pub fn authorized_by(
        &mut self,
        addr: String,
        authorization_id: u64,
        msg: &CosmosMsg,
    ) -> AuthorizedByResponse {
        self.base
            .app
            .wrap()
            .query_wasm_smart(
                &self.rbam_addr,
                &QueryMsg::AuthorizedBy {
                    addr,
                    authorization_id,
                    msg: msg.clone(),
                },
            )
            .unwrap()
    }

    pub fn test_filter(
        &mut self,
        filter: &serde_json::Value,
        msg: &CosmosMsg,
    ) -> TestFilterResponse {
        self.base
            .app
            .wrap()
            .query_wasm_smart(
                &self.rbam_addr,
                &QueryMsg::TestFilter {
                    filter: filter.clone(),
                    msg: msg.clone(),
                },
            )
            .unwrap()
    }

    // whether or not the protobuf registry has the message prepared
    pub fn protobuf_message_prepared(&mut self, message: impl Into<String>) -> bool {
        self.base
            .app
            .wrap()
            .query_wasm_smart::<cw_protobuf_registry::msg::PreparedResponse>(
                &self.protobuf_registry_addr,
                &cw_protobuf_registry::msg::QueryMsg::Prepared {
                    message_name: message.into(),
                },
            )
            .unwrap()
            .prepared
    }
}

// SUITE ASSERTIONS
impl Suite {
    pub fn assert_enabled(&mut self, expected: bool) {
        let response = self.get_enabled();
        assert_eq!(response, expected);
    }

    pub fn assert_filter(&mut self, expected: Addr) {
        let response = self.get_filter();
        assert_eq!(response, expected);
    }

    pub fn assert_protobuf_registry(&mut self, expected: Option<Addr>) {
        let response = self.get_protobuf_registry();
        assert_eq!(response, expected);
    }

    pub fn assert_role_name(&mut self, id: u64, expected_name: &str) {
        let response = self.get_role(id);
        assert_eq!(response.name, expected_name);
    }

    pub fn assert_role_metadata(&mut self, id: u64, expected: Option<String>) {
        let response = self.get_role(id);
        assert_eq!(response.metadata, expected);
    }

    pub fn assert_role_enabled(&mut self, id: u64, expected: bool) {
        let response = self.get_role(id);
        assert_eq!(response.enabled, expected);
    }

    pub fn assert_authorization_name(&mut self, id: u64, expected_name: &str) {
        let response = self.get_authorization(id);
        assert_eq!(response.authorization.name, expected_name);
    }

    pub fn assert_authorization_enabled(&mut self, id: u64, expected: bool) {
        let response = self.get_authorization(id);
        assert_eq!(response.authorization.enabled, expected);
    }

    pub fn assert_authorization_role(&mut self, authorization_id: u64, expected_role_id: u64) {
        let response = self.get_authorization(authorization_id);
        assert_eq!(response.authorization.role_id, expected_role_id);
    }

    pub fn assert_assigned(&mut self, addr: &str, role_id: u64, expected: bool) {
        let response = self.assigned(addr.to_string(), role_id);
        assert_eq!(response.assigned, expected);
    }

    pub fn assert_authorized(&mut self, addr: &str, msg: &CosmosMsg) {
        let response = self.authorized(addr.to_string(), msg, None, None);
        assert!(
            matches!(response, AuthorizedResponse::Authorized { .. }),
            "expected Authorized, got {:?}",
            response
        );
    }

    pub fn assert_authorized_by_role(&mut self, addr: &str, role_id: u64, msg: &CosmosMsg) {
        let response = self.authorized_by_role(addr.to_string(), role_id, msg, None, None);
        assert!(
            matches!(response, AuthorizedByRoleResponse::Authorized { .. }),
            "expected Authorized, got {:?}",
            response
        );
    }

    pub fn assert_authorized_by(&mut self, addr: &str, authorization_id: u64, msg: &CosmosMsg) {
        let response = self.authorized_by(addr.to_string(), authorization_id, msg);
        assert!(
            matches!(response, AuthorizedByResponse::Authorized { .. }),
            "expected Authorized, got {:?}",
            response
        );
    }

    pub fn assert_unauthorized(
        &mut self,
        addr: &str,
        msg: &CosmosMsg,
        reason: Option<impl Into<String>>,
        limit: Option<u32>,
    ) {
        let response = self.authorized(addr.to_string(), msg, None, limit);
        assert!(
            matches!(response, AuthorizedResponse::Unauthorized { .. }),
            "expected Unauthorized, got {:?}",
            response
        );
        if let Some(expected) = reason {
            match response {
                AuthorizedResponse::Unauthorized { reason, .. } => {
                    assert_eq!(reason, expected.into());
                }
                // should never happen
                _ => panic!("Expected Unauthorized response"),
            }
        }
    }

    pub fn assert_unauthorized_by_role(
        &mut self,
        addr: &str,
        role_id: u64,
        msg: &CosmosMsg,
        reason: Option<impl Into<String>>,
        limit: Option<u32>,
    ) {
        let response = self.authorized_by_role(addr.to_string(), role_id, msg, None, limit);
        assert!(
            matches!(response, AuthorizedByRoleResponse::Unauthorized { .. }),
            "expected Unauthorized, got {:?}",
            response
        );
        if let Some(expected) = reason {
            match response {
                AuthorizedByRoleResponse::Unauthorized { reason, .. } => {
                    assert_eq!(reason, expected.into());
                }
                // should never happen
                _ => panic!("Expected Unauthorized response"),
            }
        }
    }

    pub fn assert_unauthorized_by(
        &mut self,
        addr: &str,
        authorization_id: u64,
        msg: &CosmosMsg,
        reason: Option<impl Into<String>>,
    ) {
        let response = self.authorized_by(addr.to_string(), authorization_id, msg);
        assert!(
            matches!(response, AuthorizedByResponse::Unauthorized { .. }),
            "expected Unauthorized, got {:?}",
            response
        );
        if let Some(expected) = reason {
            match response {
                AuthorizedByResponse::Unauthorized { reason, .. } => {
                    assert_eq!(reason, expected.into());
                }
                // should never happen
                _ => panic!("Expected Unauthorized response"),
            }
        }
    }

    pub fn assert_filter_passes(&mut self, filter: &serde_json::Value, msg: &CosmosMsg) {
        let response = self.test_filter(filter, msg);
        assert!(
            matches!(response, TestFilterResponse::Pass { .. }),
            "expected Pass, got {:?}",
            response
        );
    }

    pub fn assert_filter_fails(
        &mut self,
        filter: &serde_json::Value,
        msg: &CosmosMsg,
        reason: Option<impl Into<String>>,
    ) {
        let response = self.test_filter(filter, msg);
        assert!(
            matches!(response, TestFilterResponse::Fail { .. }),
            "expected Fail, got {:?}",
            response
        );
        if let Some(expected) = reason {
            match response {
                TestFilterResponse::Fail { reason, .. } => {
                    assert_eq!(reason, expected.into());
                }
                // should never happen
                _ => panic!("Expected Fail response"),
            }
        }
    }

    pub fn assert_action_count(&mut self, expected: usize) {
        let response = self.list_actions(None, None, None);
        assert_eq!(response.actions.len(), expected);
    }

    pub fn assert_role_count(&mut self, expected: usize) {
        let response = self.list_roles(None, None);
        assert_eq!(response.roles.len(), expected);
    }

    pub fn assert_authorization_count(&mut self, expected: usize) {
        let response = self.list_authorizations(None, None);
        assert_eq!(response.authorizations.len(), expected);
    }

    pub fn assert_assignment_count(&mut self, expected: usize) {
        let response = self.list_assignments(None, None);
        assert_eq!(response.assignments.len(), expected);
    }

    pub fn assert_protobuf_message_prepared(&mut self, message: impl Into<String>, prepared: bool) {
        assert_eq!(self.protobuf_message_prepared(message), prepared);
    }
}

// SUITE ACTIONS
impl Suite {
    pub fn update_owner(&mut self, old_owner: impl Into<String>, new_owner: impl Into<String>) {
        let new_owner = new_owner.into();

        let msg = ExecuteMsg::UpdateOwnership(Action::TransferOwnership {
            new_owner: new_owner.clone(),
            expiry: None,
        });

        self.base
            .execute_smart_ok(old_owner, &self.rbam_addr, &msg, &[]);

        self.base.execute_smart_ok(
            new_owner,
            &self.rbam_addr,
            &ExecuteMsg::UpdateOwnership(Action::AcceptOwnership {}),
            &[],
        );
    }

    pub fn update_dao(&mut self, sender: impl Into<String>, dao: String) {
        self.base
            .execute_smart_ok(sender, &self.rbam_addr, &ExecuteMsg::UpdateDao { dao }, &[]);
    }

    pub fn update_dao_err(&mut self, sender: impl Into<String>, dao: String) -> ContractError {
        self.base
            .execute_smart_err(sender, &self.rbam_addr, &ExecuteMsg::UpdateDao { dao }, &[])
    }

    pub fn update_filter(&mut self, sender: impl Into<String>, filter: ModuleUpdate) {
        self.base.execute_smart_ok(
            sender,
            &self.rbam_addr,
            &ExecuteMsg::UpdateFilter { filter },
            &[],
        );
    }

    pub fn update_filter_err(
        &mut self,
        sender: impl Into<String>,
        filter: ModuleUpdate,
    ) -> ContractError {
        self.base.execute_smart_err(
            sender,
            &self.rbam_addr,
            &ExecuteMsg::UpdateFilter { filter },
            &[],
        )
    }

    pub fn update_protobuf_registry(
        &mut self,
        sender: impl Into<String>,
        protobuf_registry: Option<ModuleUpdate>,
    ) {
        self.base.execute_smart_ok(
            sender,
            &self.rbam_addr,
            &ExecuteMsg::UpdateProtobufRegistry { protobuf_registry },
            &[],
        );
    }

    pub fn update_protobuf_registry_err(
        &mut self,
        sender: impl Into<String>,
        protobuf_registry: Option<ModuleUpdate>,
    ) -> ContractError {
        self.base.execute_smart_err(
            sender,
            &self.rbam_addr,
            &ExecuteMsg::UpdateProtobufRegistry { protobuf_registry },
            &[],
        )
    }

    pub fn update_enabled(&mut self, sender: impl Into<String>, enabled: bool) {
        self.base.execute_smart_ok(
            sender,
            &self.rbam_addr,
            &ExecuteMsg::UpdateEnabled { enabled },
            &[],
        );
    }

    pub fn update_enabled_err(
        &mut self,
        sender: impl Into<String>,
        enabled: bool,
    ) -> ContractError {
        self.base.execute_smart_err(
            sender,
            &self.rbam_addr,
            &ExecuteMsg::UpdateEnabled { enabled },
            &[],
        )
    }

    pub fn execute_protobuf_registry(
        &mut self,
        sender: impl Into<String>,
        msg: cw_protobuf_registry::msg::ExecuteMsg,
    ) {
        self.base.execute_smart_ok(
            sender,
            &self.rbam_addr,
            &ExecuteMsg::ExecuteProtobufRegistry(msg),
            &[],
        );
    }

    pub fn execute_protobuf_registry_our_err(
        &mut self,
        sender: impl Into<String>,
        msg: cw_protobuf_registry::msg::ExecuteMsg,
    ) -> ContractError {
        self.base.execute_smart_err(
            sender,
            &self.rbam_addr,
            &ExecuteMsg::ExecuteProtobufRegistry(msg),
            &[],
        )
    }

    pub fn execute_protobuf_registry_their_err(
        &mut self,
        sender: impl Into<String>,
        msg: cw_protobuf_registry::msg::ExecuteMsg,
    ) -> cw_protobuf_registry::ContractError {
        self.base.execute_smart_err(
            sender,
            &self.rbam_addr,
            &ExecuteMsg::ExecuteProtobufRegistry(msg),
            &[],
        )
    }

    pub fn create_role(
        &mut self,
        sender: impl Into<String>,
        name: impl Into<String>,
        metadata: Option<String>,
        enabled: Option<bool>,
        authorizations: Option<Vec<InitialAuthorization>>,
        assignments: Option<Vec<String>>,
    ) -> u64 {
        let response = self.base.execute_smart_ok(
            sender,
            &self.rbam_addr,
            &ExecuteMsg::CreateRole {
                name: name.into(),
                metadata,
                enabled,
                authorizations,
                assignments,
            },
            &[],
        );

        response
            .events
            .iter()
            .find(|e| e.ty == "wasm" && e.attributes.iter().any(|a| a.key == "role_id"))
            .unwrap()
            .attributes
            .iter()
            .find(|a| a.key == "role_id")
            .unwrap()
            .value
            .parse::<u64>()
            .unwrap()
    }

    pub fn create_role_err(
        &mut self,
        sender: impl Into<String>,
        name: impl Into<String>,
        metadata: Option<String>,
        enabled: Option<bool>,
        authorizations: Option<Vec<InitialAuthorization>>,
        assignments: Option<Vec<String>>,
    ) -> ContractError {
        self.base.execute_smart_err(
            sender,
            &self.rbam_addr,
            &ExecuteMsg::CreateRole {
                name: name.into(),
                metadata,
                enabled,
                authorizations,
                assignments,
            },
            &[],
        )
    }

    pub fn update_role(
        &mut self,
        sender: impl Into<String>,
        role_id: u64,
        name: Option<String>,
        metadata: OptionalUpdate<String>,
        enabled: Option<bool>,
    ) {
        self.base.execute_smart_ok(
            sender,
            &self.rbam_addr,
            &ExecuteMsg::UpdateRole {
                role_id,
                name,
                metadata,
                enabled,
            },
            &[],
        );
    }

    pub fn update_role_err(
        &mut self,
        sender: impl Into<String>,
        role_id: u64,
        name: Option<String>,
        metadata: OptionalUpdate<String>,
        enabled: Option<bool>,
    ) -> ContractError {
        self.base.execute_smart_err(
            sender,
            &self.rbam_addr,
            &ExecuteMsg::UpdateRole {
                role_id,
                name,
                metadata,
                enabled,
            },
            &[],
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_authorization(
        &mut self,
        sender: impl Into<String>,
        role_id: u64,
        name: impl Into<String>,
        metadata: Option<String>,
        filter: Option<serde_json::Value>,
        enabled: Option<bool>,
        skip_prepare: Option<bool>,
    ) -> u64 {
        let response = self.base.execute_smart_ok(
            sender,
            &self.rbam_addr,
            &ExecuteMsg::CreateAuthorization {
                role_id,
                name: name.into(),
                metadata,
                filter,
                enabled,
                skip_prepare,
            },
            &[],
        );

        response
            .events
            .iter()
            .find(|e| e.ty == "wasm" && e.attributes.iter().any(|a| a.key == "authorization_id"))
            .unwrap()
            .attributes
            .iter()
            .find(|a| a.key == "authorization_id")
            .unwrap()
            .value
            .parse::<u64>()
            .unwrap()
    }

    pub fn create_authorization_err(
        &mut self,
        sender: impl Into<String>,
        role_id: u64,
        name: impl Into<String>,
        metadata: Option<String>,
        filter: Option<serde_json::Value>,
        enabled: Option<bool>,
    ) -> ContractError {
        self.base.execute_smart_err(
            sender,
            &self.rbam_addr,
            &ExecuteMsg::CreateAuthorization {
                role_id,
                name: name.into(),
                metadata,
                filter,
                enabled,
                skip_prepare: None,
            },
            &[],
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn update_authorization(
        &mut self,
        sender: impl Into<String>,
        authorization_id: u64,
        name: Option<String>,
        metadata: OptionalUpdate<String>,
        filter: OptionalUpdate<serde_json::Value>,
        enabled: Option<bool>,
        skip_prepare: Option<bool>,
    ) {
        self.base.execute_smart_ok(
            sender,
            &self.rbam_addr,
            &ExecuteMsg::UpdateAuthorization {
                authorization_id,
                name,
                metadata,
                filter,
                enabled,
                skip_prepare,
            },
            &[],
        );
    }

    pub fn update_authorization_err(
        &mut self,
        sender: impl Into<String>,
        authorization_id: u64,
        name: Option<String>,
        metadata: OptionalUpdate<String>,
        filter: OptionalUpdate<serde_json::Value>,
        enabled: Option<bool>,
    ) -> ContractError {
        self.base.execute_smart_err(
            sender,
            &self.rbam_addr,
            &ExecuteMsg::UpdateAuthorization {
                authorization_id,
                name,
                metadata,
                filter,
                enabled,
                skip_prepare: None,
            },
            &[],
        )
    }

    pub fn assign(&mut self, sender: impl Into<String>, assign: Vec<Assignment>) {
        self.base
            .execute_smart_ok(sender, &self.rbam_addr, &ExecuteMsg::Assign { assign }, &[]);
    }

    pub fn assign_err(
        &mut self,
        sender: impl Into<String>,
        assign: Vec<Assignment>,
    ) -> ContractError {
        self.base
            .execute_smart_err(sender, &self.rbam_addr, &ExecuteMsg::Assign { assign }, &[])
    }

    pub fn revoke(&mut self, sender: impl Into<String>, revoke: Vec<Assignment>) {
        self.base
            .execute_smart_ok(sender, &self.rbam_addr, &ExecuteMsg::Revoke { revoke }, &[]);
    }

    pub fn revoke_err(
        &mut self,
        sender: impl Into<String>,
        revoke: Vec<Assignment>,
    ) -> ContractError {
        self.base
            .execute_smart_err(sender, &self.rbam_addr, &ExecuteMsg::Revoke { revoke }, &[])
    }

    pub fn register_protobufs(
        &mut self,
        sender: impl Into<String>,
        file_descriptor_sets: Vec<Vec<u8>>,
    ) {
        self.execute_protobuf_registry(
            sender,
            cw_protobuf_registry::msg::ExecuteMsg::Register {
                file_descriptor_sets,
            },
        );
    }

    pub fn unprepare_protobuf_message(
        &mut self,
        sender: impl Into<String>,
        message: impl Into<String>,
    ) {
        self.execute_protobuf_registry(
            sender,
            cw_protobuf_registry::msg::ExecuteMsg::Unprepare {
                messages: vec![message.into()],
            },
        );
    }

    pub fn execute_actions(&mut self, sender: impl Into<String>, actions: Vec<ActionToExecute>) {
        self.base.execute_smart_ok(
            sender,
            &self.rbam_addr,
            &ExecuteMsg::ExecuteActions { actions },
            &[],
        );
    }

    pub fn execute_actions_err(
        &mut self,
        sender: impl Into<String>,
        actions: Vec<ActionToExecute>,
    ) -> ContractError {
        self.base.execute_smart_err(
            sender,
            &self.rbam_addr,
            &ExecuteMsg::ExecuteActions { actions },
            &[],
        )
    }
}
