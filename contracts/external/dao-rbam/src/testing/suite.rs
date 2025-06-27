use cosmwasm_std::{to_json_binary, Addr, CosmosMsg, Timestamp};
use cw4::Member;
use cw_ownable::Action;
use dao_interface::helpers::OptionalUpdate;
use dao_rbam::ContractError;
use dao_testing::{DaoTestingSuite, DaoTestingSuiteBase, ADDR0, ADDR1, ADDR2};

use crate::{
    action::ActionToExecute,
    msg::{
        ActionResponse, Assignment, AuthorizationResponse, DaoResponse, ExecuteMsg,
        InitialAuthorization, InitialRole, InstantiateMsg, IsAssignedRoleResponse,
        IsEnabledResponse, IsMsgAuthorizedByResponse, IsMsgAuthorizedByRoleResponse,
        IsMsgAuthorizedResponse, ListActionsResponse, ListAddressesWithRoleResponse,
        ListAssignmentsResponse, ListAuthorizationsResponse, ListProtobufFilesResponse,
        ListProtobufMessagesResponse, ListRolesForAddressResponse, ListRolesResponse, QueryMsg,
        RoleResponse, TestFilterResponse,
    },
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

        let mut suite = Suite {
            base,
            core_addr: dao.core_addr.clone(),
            rbam_addr: Addr::unchecked(""),
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

        suite
    }
}

pub struct Suite {
    pub base: DaoTestingSuiteBase,
    pub core_addr: Addr,
    pub rbam_addr: Addr,
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

    pub fn get_dao(&mut self) -> DaoResponse {
        self.base
            .app
            .wrap()
            .query_wasm_smart(&self.rbam_addr, &QueryMsg::Dao {})
            .unwrap()
    }

    pub fn get_is_enabled(&mut self) -> IsEnabledResponse {
        self.base
            .app
            .wrap()
            .query_wasm_smart(self.rbam_addr.clone(), &QueryMsg::IsEnabled {})
            .unwrap()
    }

    pub fn get_role(&mut self, id: u64) -> RoleResponse {
        self.base
            .app
            .wrap()
            .query_wasm_smart(self.rbam_addr.clone(), &QueryMsg::Role { id })
            .unwrap()
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

    pub fn is_assigned_role(&mut self, addr: String, role_id: u64) -> IsAssignedRoleResponse {
        self.base
            .app
            .wrap()
            .query_wasm_smart(
                self.rbam_addr.clone(),
                &QueryMsg::IsAssignedRole { addr, role_id },
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

    pub fn list_protobuf_files(
        &mut self,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> ListProtobufFilesResponse {
        self.base
            .app
            .wrap()
            .query_wasm_smart(
                self.rbam_addr.clone(),
                &QueryMsg::ListProtobufFiles { start_after, limit },
            )
            .unwrap()
    }

    pub fn list_protobuf_messages(
        &mut self,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> ListProtobufMessagesResponse {
        self.base
            .app
            .wrap()
            .query_wasm_smart(
                self.rbam_addr.clone(),
                &QueryMsg::ListProtobufMessages { start_after, limit },
            )
            .unwrap()
    }

    pub fn list_protobuf_messages_by_file(
        &mut self,
        file_name: String,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> ListProtobufMessagesResponse {
        self.base
            .app
            .wrap()
            .query_wasm_smart(
                self.rbam_addr.clone(),
                &QueryMsg::ListProtobufMessagesByFile {
                    file_name,
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

    pub fn is_msg_authorized(
        &mut self,
        addr: String,
        msg: &CosmosMsg,
        start_after: Option<(u64, u64)>,
        limit: Option<u32>,
    ) -> IsMsgAuthorizedResponse {
        self.base
            .app
            .wrap()
            .query_wasm_smart(
                &self.rbam_addr,
                &QueryMsg::IsMsgAuthorized {
                    addr,
                    msg: msg.clone(),
                    start_after,
                    limit,
                },
            )
            .unwrap()
    }

    pub fn is_msg_authorized_by_role(
        &mut self,
        addr: String,
        role_id: u64,
        msg: &CosmosMsg,
        start_after: Option<u64>,
        limit: Option<u32>,
    ) -> IsMsgAuthorizedByRoleResponse {
        self.base
            .app
            .wrap()
            .query_wasm_smart(
                &self.rbam_addr,
                &QueryMsg::IsMsgAuthorizedByRole {
                    addr,
                    role_id,
                    msg: msg.clone(),
                    start_after,
                    limit,
                },
            )
            .unwrap()
    }

    pub fn is_msg_authorized_by(
        &mut self,
        addr: String,
        role_id: u64,
        authorization_id: u64,
        msg: &CosmosMsg,
    ) -> IsMsgAuthorizedByResponse {
        self.base
            .app
            .wrap()
            .query_wasm_smart(
                &self.rbam_addr,
                &QueryMsg::IsMsgAuthorizedBy {
                    addr,
                    role_id,
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
}

// SUITE ASSERTIONS
impl Suite {
    pub fn assert_enabled(&mut self, expected: bool) {
        let response = self.get_is_enabled();
        assert_eq!(response.enabled, expected);
    }

    pub fn assert_role_name(&mut self, id: u64, expected_name: &str) {
        let response = self.get_role(id);
        assert_eq!(response.role.name, expected_name);
    }

    pub fn assert_role_enabled(&mut self, id: u64, expected: bool) {
        let response = self.get_role(id);
        assert_eq!(response.role.enabled, expected);
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

    pub fn assert_assigned_role(&mut self, addr: &str, role_id: u64, expected: bool) {
        let response = self.is_assigned_role(addr.to_string(), role_id);
        assert_eq!(response.assigned, expected);
    }

    pub fn assert_msg_authorized(&mut self, addr: &str, msg: &CosmosMsg) {
        let response = self.is_msg_authorized(addr.to_string(), msg, None, None);
        assert!(
            matches!(response, IsMsgAuthorizedResponse::Authorized { .. }),
            "expected Authorized, got {:?}",
            response
        );
    }

    pub fn assert_msg_authorized_by_role(&mut self, addr: &str, role_id: u64, msg: &CosmosMsg) {
        let response = self.is_msg_authorized_by_role(addr.to_string(), role_id, msg, None, None);
        assert!(
            matches!(response, IsMsgAuthorizedByRoleResponse::Authorized { .. }),
            "expected Authorized, got {:?}",
            response
        );
    }

    pub fn assert_msg_authorized_by(
        &mut self,
        addr: &str,
        role_id: u64,
        authorization_id: u64,
        msg: &CosmosMsg,
    ) {
        let response = self.is_msg_authorized_by(addr.to_string(), role_id, authorization_id, msg);
        assert!(
            matches!(response, IsMsgAuthorizedByResponse::Authorized { .. }),
            "expected Authorized, got {:?}",
            response
        );
    }

    pub fn assert_msg_unauthorized(
        &mut self,
        addr: &str,
        msg: &CosmosMsg,
        reason: Option<impl Into<String>>,
        limit: Option<u32>,
    ) {
        let response = self.is_msg_authorized(addr.to_string(), msg, None, limit);
        assert!(
            matches!(response, IsMsgAuthorizedResponse::Unauthorized { .. }),
            "expected Unauthorized, got {:?}",
            response
        );
        if let Some(expected) = reason {
            match response {
                IsMsgAuthorizedResponse::Unauthorized { reason, .. } => {
                    assert_eq!(reason, expected.into());
                }
                // should never happen
                _ => panic!("Expected Unauthorized response"),
            }
        }
    }

    pub fn assert_msg_unauthorized_by_role(
        &mut self,
        addr: &str,
        role_id: u64,
        msg: &CosmosMsg,
        reason: Option<impl Into<String>>,
        limit: Option<u32>,
    ) {
        let response = self.is_msg_authorized_by_role(addr.to_string(), role_id, msg, None, limit);
        assert!(
            matches!(response, IsMsgAuthorizedByRoleResponse::Unauthorized { .. }),
            "expected Unauthorized, got {:?}",
            response
        );
        if let Some(expected) = reason {
            match response {
                IsMsgAuthorizedByRoleResponse::Unauthorized { reason, .. } => {
                    assert_eq!(reason, expected.into());
                }
                // should never happen
                _ => panic!("Expected Unauthorized response"),
            }
        }
    }

    pub fn assert_msg_unauthorized_by(
        &mut self,
        addr: &str,
        role_id: u64,
        authorization_id: u64,
        msg: &CosmosMsg,
        reason: Option<impl Into<String>>,
    ) {
        let response = self.is_msg_authorized_by(addr.to_string(), role_id, authorization_id, msg);
        assert!(
            matches!(response, IsMsgAuthorizedByResponse::Unauthorized { .. }),
            "expected Unauthorized, got {:?}",
            response
        );
        if let Some(expected) = reason {
            match response {
                IsMsgAuthorizedByResponse::Unauthorized { reason, .. } => {
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

    pub fn assert_protobuf_files(&mut self, expected: Vec<String>) {
        let response = self.list_protobuf_files(None, None);
        assert_eq!(response.files, expected);
    }

    pub fn assert_protobuf_messages(&mut self, expected: Vec<String>) {
        let response = self.list_protobuf_messages(None, None);
        assert_eq!(response.messages, expected);
    }

    pub fn assert_protobuf_messages_by_file(&mut self, file_name: &str, expected: Vec<String>) {
        let response = self.list_protobuf_messages_by_file(file_name.to_string(), None, None);
        assert_eq!(response.messages, expected);
    }
}

// SUITE ACTIONS
impl Suite {
    pub fn set_enabled(&mut self, sender: impl Into<String>, enabled: bool) {
        self.base.execute_smart_ok(
            sender,
            &self.rbam_addr,
            &ExecuteMsg::SetEnabled { enabled },
            &[],
        );
    }

    pub fn create_role(
        &mut self,
        sender: impl Into<String>,
        name: String,
        metadata: Option<String>,
        enabled: Option<bool>,
        authorizations: Option<Vec<InitialAuthorization>>,
        assignments: Option<Vec<String>>,
    ) -> u64 {
        let response = self.base.execute_smart_ok(
            sender,
            &self.rbam_addr,
            &ExecuteMsg::CreateRole {
                name,
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

    pub fn create_authorization(
        &mut self,
        sender: impl Into<String>,
        role_id: u64,
        name: String,
        metadata: Option<String>,
        filter: Option<serde_json::Value>,
        enabled: Option<bool>,
    ) -> u64 {
        let response = self.base.execute_smart_ok(
            sender,
            &self.rbam_addr,
            &ExecuteMsg::CreateAuthorization {
                role_id,
                name,
                metadata,
                filter,
                enabled,
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

    pub fn update_authorization(
        &mut self,
        sender: impl Into<String>,
        authorization_id: u64,
        name: Option<String>,
        metadata: OptionalUpdate<String>,
        filter: OptionalUpdate<serde_json::Value>,
        enabled: Option<bool>,
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
            },
            &[],
        );
    }

    pub fn assign_roles(&mut self, sender: impl Into<String>, assign: Vec<Assignment>) {
        self.base
            .execute_smart_ok(sender, &self.rbam_addr, &ExecuteMsg::Assign { assign }, &[]);
    }

    pub fn revoke_roles(&mut self, sender: impl Into<String>, revoke: Vec<Assignment>) {
        self.base
            .execute_smart_ok(sender, &self.rbam_addr, &ExecuteMsg::Revoke { revoke }, &[]);
    }

    pub fn register_protobufs(
        &mut self,
        sender: impl Into<String>,
        file_descriptor_sets: Vec<Vec<u8>>,
    ) {
        self.base.execute_smart_ok(
            sender,
            &self.rbam_addr,
            &ExecuteMsg::RegisterProtobufs {
                file_descriptor_sets,
            },
            &[],
        );
    }

    pub fn unregister_protobufs(
        &mut self,
        sender: impl Into<String>,
        file_names: Vec<String>,
        message_limit: Option<u32>,
    ) {
        self.base.execute_smart_ok(
            sender,
            &self.rbam_addr,
            &ExecuteMsg::UnregisterProtobufs {
                file_names,
                message_limit,
            },
            &[],
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

    // Error-expected versions of actions
    pub fn set_enabled_err(&mut self, sender: impl Into<String>, enabled: bool) -> ContractError {
        self.base.execute_smart_err(
            sender,
            &self.rbam_addr,
            &ExecuteMsg::SetEnabled { enabled },
            &[],
        )
    }

    pub fn create_role_err(
        &mut self,
        sender: impl Into<String>,
        name: String,
        metadata: Option<String>,
        enabled: Option<bool>,
        authorizations: Option<Vec<InitialAuthorization>>,
        assignments: Option<Vec<String>>,
    ) -> ContractError {
        self.base.execute_smart_err(
            sender,
            &self.rbam_addr,
            &ExecuteMsg::CreateRole {
                name,
                metadata,
                enabled,
                authorizations,
                assignments,
            },
            &[],
        )
    }

    pub fn create_authorization_err(
        &mut self,
        sender: impl Into<String>,
        role_id: u64,
        name: String,
        metadata: Option<String>,
        filter: Option<serde_json::Value>,
        enabled: Option<bool>,
    ) -> ContractError {
        self.base.execute_smart_err(
            sender,
            &self.rbam_addr,
            &ExecuteMsg::CreateAuthorization {
                role_id,
                name,
                metadata,
                filter,
                enabled,
            },
            &[],
        )
    }

    pub fn assign_roles_err(
        &mut self,
        sender: impl Into<String>,
        assign: Vec<Assignment>,
    ) -> ContractError {
        self.base
            .execute_smart_err(sender, &self.rbam_addr, &ExecuteMsg::Assign { assign }, &[])
    }

    pub fn revoke_roles_err(
        &mut self,
        sender: impl Into<String>,
        revoke: Vec<Assignment>,
    ) -> ContractError {
        self.base
            .execute_smart_err(sender, &self.rbam_addr, &ExecuteMsg::Revoke { revoke }, &[])
    }

    pub fn unregister_protobufs_err(
        &mut self,
        sender: impl Into<String>,
        file_names: Vec<String>,
        message_limit: Option<u32>,
    ) -> ContractError {
        self.base.execute_smart_err(
            sender,
            &self.rbam_addr,
            &ExecuteMsg::UnregisterProtobufs {
                file_names,
                message_limit,
            },
            &[],
        )
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
