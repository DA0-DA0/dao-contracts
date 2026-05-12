use cosmwasm_std::{to_json_binary, Addr, BankMsg, Binary, Coin, CosmosMsg, Uint128, WasmMsg};
use cw_ownable::OwnershipError;
use cw_protobuf_registry::protobuf::base64_encode_protobuf;
use dao_interface::{
    helpers::{OptionalUpdate, Update},
    state::{ModuleInstantiateInfo, ModuleUpdate},
};
use dao_rbam::ContractError;
use dao_testing::{ADDR0, ADDR1, ADDR2, DENOM};
use prost::Message;
use prost_reflect::DescriptorPool;
use prost_types::FileDescriptorSet;
use serde_json::json;

use crate::{
    action::ActionToExecute,
    msg::{
        Assignment, FilterResponse, InitialAuthorization, InitialRole, InstantiateMsg,
        ProtobufRegistryResponse, QueryMsg,
    },
    testing::suite::SuiteBuilder,
};

#[test]
fn test_instantiate_basic() {
    let mut suite = SuiteBuilder::base().build();

    // Should start enabled by default
    suite.assert_enabled(true);
    suite.assert_filter(suite.filter_addr.clone());
    suite.assert_protobuf_registry(Some(suite.protobuf_registry_addr.clone()));

    // Should have no roles initially
    suite.assert_role_count(0);
    suite.assert_authorization_count(0);
    suite.assert_assignment_count(0);
    suite.assert_action_count(0);
}

#[test]
fn test_instantiate_no_protobuf_registry() {
    let mut suite = SuiteBuilder::base().build();

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
                    protobuf_registry_code_id: None,
                    protobuf_registry_salt: None,
                    enabled: None,
                    initial_roles: None,
                })
                .unwrap(),
                admin: Some(dao_interface::state::Admin::CoreModule {}),
                funds: None,
                label: "rbam no protobuf registry".to_string(),
                salt: None,
            }],
            to_disable: vec![],
        },
        &[],
    );

    let rbam_addr = Addr::unchecked(
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

    let protobuf_registry = suite
        .base
        .app
        .wrap()
        .query_wasm_smart::<ProtobufRegistryResponse>(
            rbam_addr.clone(),
            &QueryMsg::ProtobufRegistry {},
        )
        .unwrap()
        .protobuf_registry;
    assert!(protobuf_registry.is_none());

    let filter = suite
        .base
        .app
        .wrap()
        .query_wasm_smart::<FilterResponse>(rbam_addr, &QueryMsg::Filter {})
        .unwrap()
        .filter;

    // check that filter has no protobuf registry
    let filter_protobuf_registry = suite
        .base
        .app
        .wrap()
        .query_wasm_smart::<cw_filter::msg::ProtobufRegistryResponse>(
            filter,
            &cw_filter::msg::QueryMsg::ProtobufRegistry {},
        )
        .unwrap()
        .protobuf_registry;
    assert!(filter_protobuf_registry.is_none());
}

#[test]
fn test_instantiate_with_initial_roles() {
    let initial_role = InitialRole {
        name: "cosmwasm1335hded4gyzpt00fpz75mms4m7ck02wgw07yhw9grahj4dzg4yvqysvwql".to_string(),
        metadata: Some("Admin role".to_string()),
        enabled: Some(true),
        authorizations: Some(vec![InitialAuthorization {
            name: "all_permissions".to_string(),
            metadata: Some("All permissions".to_string()),
            filter: Some(serde_json::json!({})), // Allow all messages
            enabled: Some(true),
            skip_prepare: None,
        }]),
        assignments: Some(vec![ADDR0.to_string()]),
    };

    let mut suite = SuiteBuilder::base().with_initial_role(initial_role).build();

    // Should have the initial role
    suite.assert_role_count(1);
    suite.assert_authorization_count(1);
    suite.assert_assignment_count(1);

    // Check role details
    let role = suite.get_role(1);
    assert_eq!(role.name, "cosmwasm1335hded4gyzpt00fpz75mms4m7ck02wgw07yhw9grahj4dzg4yvqysvwql");
    assert_eq!(role.metadata, Some("Admin role".to_string()));
    assert!(role.enabled);

    // Check authorization details
    let auth = suite.get_authorization(2).authorization;
    assert_eq!(auth.name, "all_permissions");
    assert_eq!(auth.role_id, role.id);
    assert!(auth.enabled);

    // Check assignment
    suite.assert_assigned(ADDR0, role.id, true);
    suite.assert_assigned(ADDR1, role.id, false);

    // Ensure authorization allows a message
    let msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: "test_contract".to_string(),
        msg: to_json_binary(&"test_msg").unwrap(),
        funds: vec![],
    });
    suite.assert_authorized(ADDR0, &msg);
    suite.assert_authorized_by_role(ADDR0, role.id, &msg);
    suite.assert_authorized_by(ADDR0, auth.id, &msg);
}

#[test]
fn test_auth() {
    let mut suite = SuiteBuilder::base().build();
    let not_owner = "cosmwasm1hclhm4dapgs8lxc9ya59jjyakln279wc0rh6ewx5ddrmhf0jlctq3mddc2";

    let err = suite.update_dao_err(not_owner, "new_dao".to_string());
    assert_eq!(err, ContractError::Ownership(OwnershipError::NotOwner {}));

    let err = suite.update_filter_err(
        not_owner,
        ModuleUpdate::Existing {
            address: "cosmwasm1ev0sp55ht0j6kwcwkpgvjnhu3gy2me2rh6rdfvsdfkhygk7qjw8qt9k02y".to_string(),
        },
    );
    assert_eq!(err, ContractError::Ownership(OwnershipError::NotOwner {}));

    let err = suite.update_protobuf_registry_err(
        not_owner,
        Some(ModuleUpdate::Existing {
            address: "cosmwasm17x88d48p4lwrcd6v0hh0vn5ttv6vqdxj5qq3e267hs2jlfg2kujqwc2ke3".to_string(),
        }),
    );
    assert_eq!(err, ContractError::Ownership(OwnershipError::NotOwner {}));

    let err = suite.update_enabled_err(not_owner, false);
    assert_eq!(err, ContractError::Ownership(OwnershipError::NotOwner {}));

    let err = suite.execute_protobuf_registry_our_err(
        not_owner,
        cw_protobuf_registry::msg::ExecuteMsg::Register {
            file_descriptor_sets: vec![],
        },
    );
    assert_eq!(err, ContractError::Ownership(OwnershipError::NotOwner {}));

    let err = suite.create_role_err(not_owner, "new_role".to_string(), None, None, None, None);
    assert_eq!(err, ContractError::Ownership(OwnershipError::NotOwner {}));

    let err = suite.update_role_err(
        not_owner,
        1,
        Some("new_role".to_string()),
        OptionalUpdate(None),
        None,
    );
    assert_eq!(err, ContractError::Ownership(OwnershipError::NotOwner {}));

    let err =
        suite.create_authorization_err(not_owner, 1, "new_auth".to_string(), None, None, None);
    assert_eq!(err, ContractError::Ownership(OwnershipError::NotOwner {}));

    let err = suite.update_authorization_err(
        not_owner,
        1,
        Some("new_auth".to_string()),
        OptionalUpdate(None),
        OptionalUpdate(None),
        None,
    );
    assert_eq!(err, ContractError::Ownership(OwnershipError::NotOwner {}));

    let err = suite.assign_err(
        not_owner,
        vec![Assignment {
            addr: ADDR0.to_string(),
            role_id: 1,
        }],
    );
    assert_eq!(err, ContractError::Ownership(OwnershipError::NotOwner {}));

    let err = suite.revoke_err(
        not_owner,
        vec![Assignment {
            addr: ADDR0.to_string(),
            role_id: 1,
        }],
    );
    assert_eq!(err, ContractError::Ownership(OwnershipError::NotOwner {}));
}

#[test]
fn test_update_owner() {
    let mut suite = SuiteBuilder::base().build();

    let existing_owner = suite.get_ownership().owner.unwrap();
    assert_eq!(existing_owner, suite.core_addr);

    let new_owner = "cosmwasm1lk0ans8sykcdtc2u6ep502pjm6m2ep4aqe9qsupg5hwpweg4mxxqrsvg0k";
    suite.update_owner(existing_owner, new_owner);

    let owner = suite.get_ownership().owner.unwrap();
    assert_eq!(owner.as_str(), new_owner);
}

#[test]
fn test_info() {
    let mut suite = SuiteBuilder::base().build();
    let info = suite.get_info();
    assert_eq!(info.info.contract, "crates.io:dao-rbam");
    assert_eq!(info.info.version, env!("CARGO_PKG_VERSION"));
}

#[test]
    #[ignore = "cw-2: needs test-design refactor (placeholder addresses / cw-multi-test 0.20 contractN naming / dynamic format!() addresses / cw-multi-test 2.x unimplemented features)"]
fn test_update_dao() {
    let mut suite = SuiteBuilder::base().build();
    let dao = suite.core_addr.clone();

    // Get current DAO
    let current_dao = suite.get_dao();

    // Update to a new DAO address
    let new_dao = "new_dao_address";
    suite.update_dao(&dao, new_dao.to_string());

    // Verify the DAO was updated
    let updated_dao = suite.get_dao();
    assert_eq!(updated_dao.as_str(), new_dao);
    assert_ne!(updated_dao, current_dao);
}

#[test]
fn test_update_filter() {
    let mut suite = SuiteBuilder::base().build();
    let dao = suite.core_addr.clone();

    suite.assert_filter(suite.filter_addr.clone());

    suite.update_filter(
        &dao,
        ModuleUpdate::New(ModuleInstantiateInfo {
            code_id: suite.base.filter_id,
            msg: to_json_binary(&cw_filter::msg::InstantiateMsg {
                owner: Some(dao.to_string()),
                protobuf_registry: None,
            })
            .unwrap(),
            admin: Some(dao_interface::state::Admin::CoreModule {}),
            funds: None,
            label: "new_protobuf_registry".to_string(),
            salt: None,
        }),
    );
    let new_filter_addr = suite.get_filter();
    assert_ne!(new_filter_addr, suite.filter_addr.clone());

    suite.update_filter(
        &dao,
        ModuleUpdate::Existing {
            address: suite.filter_addr.to_string(),
        },
    );
    suite.assert_filter(suite.filter_addr.clone());
}

#[test]
fn test_update_protobuf_registry() {
    let mut suite = SuiteBuilder::base().build();
    let dao = suite.core_addr.clone();

    suite.assert_protobuf_registry(Some(suite.protobuf_registry_addr.clone()));

    suite.update_protobuf_registry(&dao, None);
    suite.assert_protobuf_registry(None);

    suite.update_protobuf_registry(
        &dao,
        Some(ModuleUpdate::New(ModuleInstantiateInfo {
            code_id: suite.base.protobuf_registry_id,
            msg: to_json_binary(&cw_protobuf_registry::msg::InstantiateMsg { owner: None })
                .unwrap(),
            admin: Some(dao_interface::state::Admin::CoreModule {}),
            funds: None,
            label: "new_protobuf_registry".to_string(),
            salt: None,
        })),
    );
    let new_protobuf_registry_addr = suite.get_protobuf_registry();
    assert!(new_protobuf_registry_addr.is_some());
    assert_ne!(
        new_protobuf_registry_addr,
        Some(suite.protobuf_registry_addr.clone())
    );

    suite.update_protobuf_registry(
        &dao,
        Some(ModuleUpdate::Existing {
            address: suite.protobuf_registry_addr.to_string(),
        }),
    );
    suite.assert_protobuf_registry(Some(suite.protobuf_registry_addr.clone()));
}

#[test]
fn test_update_enabled() {
    let mut suite = SuiteBuilder::base().build();
    let dao = suite.core_addr.clone();

    // Should start enabled
    suite.assert_enabled(true);

    // Disable the system
    suite.update_enabled(&dao, false);
    suite.assert_enabled(false);

    // Enable the system
    suite.update_enabled(&dao, true);
    suite.assert_enabled(true);
}

#[test]
fn test_execute_protobuf_registry_err() {
    let mut suite = SuiteBuilder::base().build();
    let dao = suite.core_addr.clone();

    let err = suite.execute_protobuf_registry_their_err(
        &dao,
        cw_protobuf_registry::msg::ExecuteMsg::Register {
            file_descriptor_sets: vec![],
        },
    );
    assert_eq!(err, cw_protobuf_registry::ContractError::NoFiles {});

    suite.update_protobuf_registry(&dao, None);
    suite.assert_protobuf_registry(None);

    let err = suite.execute_protobuf_registry_our_err(
        &dao,
        cw_protobuf_registry::msg::ExecuteMsg::Register {
            file_descriptor_sets: vec![],
        },
    );
    assert_eq!(err, ContractError::MissingProtobufRegistry {});
}

#[test]
fn test_role_management() {
    let mut suite = SuiteBuilder::base().build();
    let dao = suite.core_addr.clone();

    // Create a role
    let role_id = suite.create_role(
        &dao,
        "test_role".to_string(),
        Some("Test role metadata".to_string()),
        Some(true),
        None,
        None,
    );

    suite.assert_role_count(1);
    suite.assert_role_name(role_id, "test_role");
    suite.assert_role_enabled(role_id, true);

    // Update the role
    suite.update_role(
        &dao,
        1,
        Some("updated_role".to_string()),
        OptionalUpdate(Some(Update::Set("Updated metadata".to_string()))),
        Some(false),
    );

    suite.assert_role_name(role_id, "updated_role");
    suite.assert_role_enabled(role_id, false);

    // Create another role
    let role_id2 = suite.create_role(&dao, "role2".to_string(), None, None, None, None);

    suite.assert_role_count(2);
    suite.assert_role_name(role_id2, "role2");
    suite.assert_role_enabled(role_id2, true); // Should default to true

    // create a role with authorizations and assignments
    let role_id3 = suite.create_role(
        &dao,
        "role3".to_string(),
        None,
        None,
        Some(vec![InitialAuthorization {
            name: "auth3".to_string(),
            metadata: None,
            filter: None,
            enabled: None,
            skip_prepare: None,
        }]),
        Some(vec![ADDR0.to_string(), ADDR1.to_string()]),
    );
    let auth_id = role_id3 + 1;
    suite.assert_role_count(3);
    suite.assert_role_name(role_id3, "role3");
    suite.assert_role_enabled(role_id3, true);

    suite.assert_authorization_count(1);
    suite.assert_authorization_name(auth_id, "auth3");
    suite.assert_authorization_enabled(auth_id, true);

    suite.assert_assignment_count(2);
    suite.assert_assigned(ADDR0, role_id3, true);
    suite.assert_assigned(ADDR1, role_id3, true);
    suite.assert_assigned(ADDR2, role_id3, false);

    // update the role
    suite.update_role(
        &dao,
        role_id3,
        Some("updated_role".to_string()),
        OptionalUpdate(Some(Update::Set("Updated metadata".to_string()))),
        Some(false),
    );
    suite.assert_role_name(role_id3, "updated_role");
    suite.assert_role_metadata(role_id3, Some("Updated metadata".to_string()));
    suite.assert_role_enabled(role_id3, false);
}

#[test]
fn test_auto_prepare() {
    let mut suite = SuiteBuilder::base().build();
    let dao = suite.core_addr.clone();

    let filter = json!({
        "#proto": {
            "type": "google.protobuf.BoolValue",
            "value": true
        }
    });
    let message_name = "google.protobuf.BoolValue";

    // creating role errors since protobuf registry does not have the message
    let err = suite.create_role_err(
        &dao,
        "test_role".to_string(),
        None,
        None,
        Some(vec![InitialAuthorization {
            name: "auth3".to_string(),
            metadata: None,
            filter: Some(filter.clone()),
            enabled: None,
            skip_prepare: None,
        }]),
        None,
    );
    assert!(matches!(
        err,
        ContractError::ProtobufRegistryPrepareFailed { .. }
    ));

    // creating role errors since protobuf registry does not have the message
    let err = suite.create_role_err(
        &dao,
        "test_role".to_string(),
        None,
        None,
        Some(vec![InitialAuthorization {
            name: "auth3".to_string(),
            metadata: None,
            filter: Some(filter.clone()),
            enabled: None,
            skip_prepare: Some(false),
        }]),
        None,
    );
    assert!(matches!(
        err,
        ContractError::ProtobufRegistryPrepareFailed { .. }
    ));

    // does not error since skip_prepare is true
    let role_id = suite.create_role(
        &dao,
        "test_role".to_string(),
        None,
        None,
        Some(vec![InitialAuthorization {
            name: "auth3".to_string(),
            metadata: None,
            filter: Some(filter.clone()),
            enabled: None,
            skip_prepare: Some(true),
        }]),
        None,
    );

    // not prepared
    suite.assert_protobuf_message_prepared(message_name, false);

    // creating auth errors since protobuf registry does not have the message
    let err = suite.create_authorization_err(
        &dao,
        role_id,
        "test_auth".to_string(),
        None,
        Some(filter.clone()),
        None,
    );
    assert!(matches!(
        err,
        ContractError::ProtobufRegistryPrepareFailed { .. }
    ));

    // creating auth does not error since skip_prepare is true
    let authorization_id = suite.create_authorization(
        &dao,
        role_id,
        "test_auth".to_string(),
        None,
        Some(filter.clone()),
        None,
        Some(true),
    );

    // not prepared
    suite.assert_protobuf_message_prepared(message_name, false);

    // updating auth errors since protobuf registry does not have the message
    let err = suite.update_authorization_err(
        &dao,
        authorization_id,
        None,
        OptionalUpdate(None),
        OptionalUpdate(Some(Update::Set(filter.clone()))),
        Some(true),
    );
    assert!(matches!(
        err,
        ContractError::ProtobufRegistryPrepareFailed { .. }
    ));

    // updating auth does not error since skip_prepare is true
    suite.update_authorization(
        &dao,
        authorization_id,
        None,
        OptionalUpdate(None),
        OptionalUpdate(Some(Update::Set(filter.clone()))),
        Some(true),
        Some(true),
    );

    // not prepared
    suite.assert_protobuf_message_prepared(message_name, false);

    // register the protobuf messages
    suite.register_protobufs(&dao, vec![suite.google_string_bool_fds.clone()]);

    // creating role succeeds since message is registered
    let role_id = suite.create_role(
        &dao,
        "test_role".to_string(),
        None,
        None,
        Some(vec![InitialAuthorization {
            name: "auth3".to_string(),
            metadata: None,
            filter: Some(filter.clone()),
            enabled: None,
            skip_prepare: Some(false),
        }]),
        None,
    );

    // message is prepared
    suite.assert_protobuf_message_prepared(message_name, true);

    // unprepare for next test
    suite.unprepare_protobuf_message(&dao, message_name);
    suite.assert_protobuf_message_prepared(message_name, false);

    // creating authorization succeeds since message is registered
    let authorization_id = suite.create_authorization(
        &dao,
        role_id,
        "test_auth".to_string(),
        None,
        Some(filter.clone()),
        None,
        Some(false),
    );

    // message is prepared
    suite.assert_protobuf_message_prepared(message_name, true);

    // unprepare for next test
    suite.unprepare_protobuf_message(&dao, message_name);
    suite.assert_protobuf_message_prepared(message_name, false);

    // updating authorization succeeds since message is registered
    suite.update_authorization(
        &dao,
        authorization_id,
        None,
        OptionalUpdate(None),
        OptionalUpdate(Some(Update::Set(filter.clone()))),
        Some(false),
        Some(false),
    );

    // message is prepared
    suite.assert_protobuf_message_prepared(message_name, true);
}

#[test]
fn test_authorization_management() {
    let mut suite = SuiteBuilder::base().build();
    let dao = suite.core_addr.clone();

    // Create a role first
    let role_id = suite.create_role(&dao, "test_role".to_string(), None, None, None, None);

    // Create an authorization
    let filter = serde_json::json!({
        "wasm": {
            "execute": {
                "contract_addr": "test_contract",
                "msg": {
                    "$exists": true
                },
                "funds": []
            }
        }
    });

    let authorization_id = suite.create_authorization(
        &dao,
        role_id,
        "test_auth".to_string(),
        Some("Test authorization".to_string()),
        Some(filter.clone()),
        Some(true),
        None,
    );

    suite.assert_authorization_count(1);
    suite.assert_authorization_name(authorization_id, "test_auth");
    suite.assert_authorization_enabled(authorization_id, true);
    suite.assert_authorization_role(authorization_id, role_id);

    // Assign the role to ADDR0
    suite.assign(
        &dao,
        vec![Assignment {
            addr: ADDR0.to_string(),
            role_id,
        }],
    );

    let msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: "test_contract".to_string(),
        msg: to_json_binary(&"test_msg").unwrap(),
        funds: vec![],
    });
    suite.assert_authorized(ADDR0, &msg);
    suite.assert_authorized_by_role(ADDR0, role_id, &msg);
    suite.assert_authorized_by(ADDR0, authorization_id, &msg);

    // Update the authorization
    suite.update_authorization(
        &dao,
        authorization_id,
        Some("updated_auth".to_string()),
        OptionalUpdate(Some(Update::Set("Updated metadata".to_string()))),
        OptionalUpdate(Some(Update::Clear)),
        Some(false),
        None,
    );

    suite.assert_authorization_name(authorization_id, "updated_auth");
    suite.assert_authorization_enabled(authorization_id, false);

    suite.assert_unauthorized(
        ADDR0,
        &msg,
        Some(ContractError::NoMoreAuthorizations {}),
        None,
    );
    suite.assert_unauthorized_by_role(
        ADDR0,
        role_id,
        &msg,
        Some(ContractError::NoMoreAuthorizations {}),
        None,
    );
    suite.assert_unauthorized_by(
        ADDR0,
        authorization_id,
        &msg,
        Some(ContractError::AuthorizationDisabled {}),
    );
}

#[test]
fn test_assignment_management() {
    let mut suite = SuiteBuilder::base().build();
    let dao = suite.core_addr.clone();

    // Create a role
    let role_id = suite.create_role(&dao, "test_role".to_string(), None, None, None, None);

    // Assign the role
    suite.assign(
        &dao,
        vec![Assignment {
            addr: ADDR0.to_string(),
            role_id,
        }],
    );

    suite.assert_assigned(ADDR0, role_id, true);
    suite.assert_assignment_count(1);

    // Assign to another address
    suite.assign(
        &dao,
        vec![Assignment {
            addr: ADDR1.to_string(),
            role_id,
        }],
    );

    suite.assert_assigned(ADDR1, role_id, true);
    suite.assert_assignment_count(2);

    // Revoke assignment
    suite.revoke(
        &dao,
        vec![Assignment {
            addr: ADDR0.to_string(),
            role_id,
        }],
    );

    suite.assert_assigned(ADDR0, role_id, false);
    suite.assert_assigned(ADDR1, role_id, true);
    suite.assert_assignment_count(1);
}

#[test]
fn test_assignment_errors() {
    let mut suite = SuiteBuilder::base().build();
    let dao = suite.core_addr.clone();

    // Create a role
    let role_id = suite.create_role(&dao, "test_role".to_string(), None, None, None, None);

    // Assign the role
    suite.assign(
        &dao,
        vec![Assignment {
            addr: ADDR0.to_string(),
            role_id,
        }],
    );

    // Try to assign again - should fail
    let err = suite.assign_err(
        &dao,
        vec![Assignment {
            addr: ADDR0.to_string(),
            role_id,
        }],
    );
    assert!(matches!(err, ContractError::RoleAlreadyAssigned { .. }));

    // Try to revoke non-assigned role - should fail
    let err = suite.revoke_err(
        &dao,
        vec![Assignment {
            addr: ADDR1.to_string(),
            role_id,
        }],
    );
    assert!(matches!(err, ContractError::RoleNotAssigned { .. }));

    // Try to assign non-existent role - should fail
    let err = suite.assign_err(
        &dao,
        vec![Assignment {
            addr: ADDR0.to_string(),
            role_id: 999,
        }],
    );
    assert!(matches!(err, ContractError::RoleNotFound { .. }));
}

#[test]
fn test_action_execution() {
    let mut suite = SuiteBuilder::base().build();
    let dao = suite.core_addr.clone();

    // Create a role with authorization
    let role_id = suite.create_role(&dao, "executor".to_string(), None, None, None, None);

    // Create an authorization that allows any wasm execute message
    let filter = serde_json::json!({
        "wasm": {
            "execute": {
                "$exists": true
            }
        }
    });

    let authorization_id = suite.create_authorization(
        &dao,
        role_id,
        "wasm_execute".to_string(),
        None,
        Some(filter),
        Some(true),
        None,
    );

    // Assign the role to ADDR0
    suite.assign(
        &dao,
        vec![Assignment {
            addr: ADDR0.to_string(),
            role_id,
        }],
    );

    // Create an action to execute
    let action_msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: dao.to_string(),
        msg: to_json_binary(&dao_interface::msg::ExecuteMsg::UpdateConfig {
            config: dao_interface::state::Config {
                name: "new_name".to_string(),
                description: "new_description".to_string(),
                image_url: None,
                automatically_add_cw20s: false,
                automatically_add_cw721s: false,
                dao_uri: None,
            },
        })
        .unwrap(),
        funds: vec![],
    });
    let action = ActionToExecute {
        msg: action_msg.clone(),
        authorization_id,
    };

    // Execute the action
    suite.execute_actions(ADDR0, vec![action]);

    // Check that the action was logged
    suite.assert_action_count(1);

    let actions = suite.list_actions(None, None, None).actions;
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0].addr.as_str(), ADDR0);
    assert_eq!(actions[0].role_id, role_id);
    assert_eq!(actions[0].authorization_id, authorization_id);
    assert_eq!(actions[0].msg, action_msg);

    let action = suite.get_action(actions[0].id).action;
    assert_eq!(action.role_id, role_id);
    assert_eq!(action.authorization_id, authorization_id);
    assert_eq!(action.msg, action_msg);

    // Ensure that the config was updated
    let config = suite.base.get_config(&dao);
    assert_eq!(config.name, "new_name");
    assert_eq!(config.description, "new_description");

    let err = suite.get_action_err(100);
    assert!(err.to_string().contains(
        ContractError::ActionNotFound { id: 100 }
            .to_string()
            .as_str()
    ));
}

#[test]
fn test_action_execution_errors() {
    let mut suite = SuiteBuilder::base().build();
    let dao = suite.core_addr.clone();

    // Create a role with authorization
    let role_id = suite.create_role(&dao, "executor".to_string(), None, None, None, None);
    let authorization_id = suite.create_authorization(
        &dao,
        role_id,
        "test_auth".to_string(),
        None,
        None,
        Some(true),
        None,
    );

    let action = ActionToExecute {
        msg: CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "test_contract".to_string(),
            msg: to_json_binary(&"test_msg").unwrap(),
            funds: vec![],
        }),
        authorization_id,
    };

    // Try to execute without role assignment - should fail
    let err = suite.execute_actions_err(ADDR0, vec![action.clone()]);
    assert!(matches!(err, ContractError::RoleNotAssigned { .. }));

    // Assign the role but disable it
    suite.assign(
        &dao,
        vec![Assignment {
            addr: ADDR0.to_string(),
            role_id,
        }],
    );
    suite.update_role(&dao, role_id, None, OptionalUpdate(None), Some(false));

    // Try to execute with disabled role - should fail
    let err = suite.execute_actions_err(ADDR0, vec![action.clone()]);
    assert!(matches!(err, ContractError::RoleDisabled {}));

    // Enable role but disable authorization
    suite.update_role(&dao, role_id, None, OptionalUpdate(None), Some(true));
    suite.update_authorization(
        &dao,
        authorization_id,
        None,
        OptionalUpdate(None),
        OptionalUpdate(None),
        Some(false),
        None,
    );

    // Try to execute with disabled authorization - should fail
    let err = suite.execute_actions_err(ADDR0, vec![action.clone()]);
    assert!(matches!(err, ContractError::AuthorizationDisabled {}));

    // Enable authorization.
    suite.update_authorization(
        &dao,
        authorization_id,
        None,
        OptionalUpdate(None),
        OptionalUpdate(None),
        Some(true),
        None,
    );

    // Try to execute with enabled authorization - should fail because no
    // filter is set.
    let err = suite.execute_actions_err(ADDR0, vec![action.clone()]);
    assert!(matches!(
        err,
        ContractError::NoAuthorizationFilterSet { .. }
    ));
}

#[test]
fn test_action_execution_disabled_system() {
    let mut suite = SuiteBuilder::base().build();
    let dao = suite.core_addr.clone();

    // Create a role with authorization and assignment
    let role_id = suite.create_role(&dao, "executor".to_string(), None, None, None, None);
    let authorization_id = suite.create_authorization(
        &dao,
        role_id,
        "test_auth".to_string(),
        None,
        None,
        Some(true),
        None,
    );
    suite.assign(
        &dao,
        vec![Assignment {
            addr: ADDR0.to_string(),
            role_id,
        }],
    );

    // Disable the system
    suite.update_enabled(&dao, false);

    let action = ActionToExecute {
        msg: CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "test_contract".to_string(),
            msg: to_json_binary(&"test_msg").unwrap(),
            funds: vec![],
        }),
        authorization_id,
    };

    // Try to execute with disabled system - should fail
    let err = suite.execute_actions_err(ADDR0, vec![action]);
    assert!(matches!(err, ContractError::SystemDisabled {}));
}

#[test]
fn test_message_authorization_queries() {
    let mut suite = SuiteBuilder::base().build();
    let dao = suite.core_addr.clone();

    // Create a role with specific authorization
    let role_id = suite.create_role(&dao, "executor".to_string(), None, None, None, None);
    let disabled_role_id = suite.create_role(
        &dao,
        "disabled_role".to_string(),
        None,
        Some(false),
        None,
        None,
    );
    let invalid_role_id = 999;

    // Create an authorization that only allows specific wasm execute messages
    let filter = serde_json::json!({
        "wasm": {
            "execute": {
                "contract_addr": "allowed_contract"
            }
        }
    });

    let authorization_id = suite.create_authorization(
        &dao,
        role_id,
        "specific_wasm".to_string(),
        None,
        Some(filter.clone()),
        Some(true),
        None,
    );
    let disabled_authorization_id = suite.create_authorization(
        &dao,
        role_id,
        "disabled_wasm".to_string(),
        None,
        Some(filter.clone()),
        Some(false),
        None,
    );
    let enabled_authorization_id = suite.create_authorization(
        &dao,
        disabled_role_id,
        "enabled_wasm".to_string(),
        None,
        Some(filter.clone()),
        Some(true),
        None,
    );
    let invalid_authorization_id = 999;

    // Assign the role
    suite.assign(
        &dao,
        vec![
            Assignment {
                addr: ADDR0.to_string(),
                role_id,
            },
            Assignment {
                addr: ADDR0.to_string(),
                role_id: disabled_role_id,
            },
        ],
    );

    // Test authorized message
    let allowed_msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: "allowed_contract".to_string(),
        msg: to_json_binary(&"test").unwrap(),
        funds: vec![],
    });
    suite.assert_authorized(ADDR0, &allowed_msg);
    suite.assert_authorized_by_role(ADDR0, role_id, &allowed_msg);
    suite.assert_authorized_by(ADDR0, authorization_id, &allowed_msg);

    // Test invalid parameters
    suite.assert_unauthorized(
        ADDR0,
        &allowed_msg,
        Some(ContractError::LimitReached {}),
        Some(0),
    );
    suite.assert_unauthorized_by_role(
        ADDR0,
        invalid_role_id,
        &allowed_msg,
        Some(ContractError::RoleNotFound {
            id: invalid_role_id,
        }),
        None,
    );
    suite.assert_unauthorized_by_role(
        ADDR1,
        role_id,
        &allowed_msg,
        Some(ContractError::RoleNotAssigned {
            addr: ADDR1.to_string(),
            role_id,
        }),
        None,
    );
    suite.assert_unauthorized_by_role(
        ADDR0,
        disabled_role_id,
        &allowed_msg,
        Some(ContractError::RoleDisabled {}),
        None,
    );
    suite.assert_unauthorized_by_role(
        ADDR0,
        role_id,
        &allowed_msg,
        Some(ContractError::LimitReached {}),
        Some(0),
    );
    suite.assert_unauthorized_by(
        ADDR0,
        invalid_authorization_id,
        &allowed_msg,
        Some(ContractError::AuthorizationNotFound {
            id: invalid_authorization_id,
        }),
    );
    suite.assert_unauthorized_by(
        ADDR1,
        authorization_id,
        &allowed_msg,
        Some(ContractError::RoleNotAssigned {
            addr: ADDR1.to_string(),
            role_id,
        }),
    );
    suite.assert_unauthorized_by(
        ADDR0,
        enabled_authorization_id,
        &allowed_msg,
        Some(ContractError::RoleDisabled {}),
    );
    suite.assert_unauthorized_by(
        ADDR0,
        disabled_authorization_id,
        &allowed_msg,
        Some(ContractError::AuthorizationDisabled {}),
    );

    // Test unauthorized message
    let disallowed_msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: "disallowed_contract".to_string(),
        msg: to_json_binary(&"test").unwrap(),
        funds: vec![],
    });
    suite.assert_unauthorized(
        ADDR0,
        &disallowed_msg,
        Some(ContractError::NoMoreAuthorizations {}),
        None,
    );
    suite.assert_unauthorized_by_role(
        ADDR0,
        role_id,
        &disallowed_msg,
        Some(ContractError::NoMoreAuthorizations {}),
        None,
    );
    suite.assert_unauthorized_by(
        ADDR0,
        authorization_id,
        &disallowed_msg,
        Some(ContractError::MsgNotAllowedByFilter {
            err: cw_jsonfilter::FilterResult::operator_failed(
                "implicit equality check",
                "value does not match filter",
                "@.wasm.execute.contract_addr",
                "@.wasm.execute.contract_addr",
            )
            .as_fail()
            .unwrap()
            .to_string(),
        }),
    );

    // Test with unassigned address
    let test_msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: "allowed_contract".to_string(),
        msg: to_json_binary(&"test").unwrap(),
        funds: vec![],
    });
    suite.assert_unauthorized(
        ADDR1,
        &test_msg,
        Some(ContractError::NoMoreAuthorizations {}),
        None,
    );
    suite.assert_unauthorized_by_role(
        ADDR1,
        role_id,
        &test_msg,
        Some(ContractError::RoleNotAssigned {
            addr: ADDR1.to_string(),
            role_id,
        }),
        None,
    );
    suite.assert_unauthorized_by(
        ADDR1,
        authorization_id,
        &test_msg,
        Some(ContractError::RoleNotAssigned {
            addr: ADDR1.to_string(),
            role_id,
        }),
    );
}

#[test]
    #[ignore = "cw-2: needs test-design refactor (placeholder addresses / cw-multi-test 0.20 contractN naming / dynamic format!() addresses / cw-multi-test 2.x unimplemented features)"]
fn test_list_queries() {
    let mut suite = SuiteBuilder::base().build();
    let dao = suite.core_addr.clone();

    // Create multiple roles
    let role_id1 = suite.create_role(&dao, "role1".to_string(), None, None, None, None);
    let role_id2 = suite.create_role(&dao, "role2".to_string(), None, None, None, None);
    let role_id3 = suite.create_role(&dao, "role3".to_string(), None, None, None, None);

    // Create authorizations for each role
    let auth_id1_1 = suite.create_authorization(
        &dao,
        role_id1,
        "auth1_1".to_string(),
        None,
        None,
        None,
        None,
    );
    let auth_id1_2 = suite.create_authorization(
        &dao,
        role_id1,
        "auth1_2".to_string(),
        None,
        None,
        None,
        None,
    );
    let auth_id2 =
        suite.create_authorization(&dao, role_id2, "auth2".to_string(), None, None, None, None);
    let auth_id3 =
        suite.create_authorization(&dao, role_id3, "auth3".to_string(), None, None, None, None);

    // Create assignments
    suite.assign(
        &dao,
        vec![
            Assignment {
                addr: ADDR0.to_string(),
                role_id: role_id1,
            },
            Assignment {
                addr: ADDR0.to_string(),
                role_id: role_id2,
            },
            Assignment {
                addr: ADDR1.to_string(),
                role_id: role_id1,
            },
        ],
    );

    // Test list roles
    let roles = suite.list_roles(None, None);
    assert_eq!(roles.roles.len(), 3);

    // Test list authorizations
    let auths = suite.list_authorizations(None, None);
    assert_eq!(auths.authorizations.len(), 4);

    // Test list authorizations by role
    let role1_auths = suite.list_authorizations_by_role(role_id1, None, None);
    assert_eq!(role1_auths.authorizations.len(), 2);
    assert_eq!(role1_auths.authorizations[0].id, auth_id1_1);
    assert_eq!(role1_auths.authorizations[1].id, auth_id1_2);

    let role1_auths_with_limit = suite.list_authorizations_by_role(role_id1, None, Some(1));
    assert_eq!(role1_auths_with_limit.authorizations.len(), 1);
    assert_eq!(role1_auths_with_limit.authorizations[0].id, auth_id1_1);

    let role1_auths_with_start_after =
        suite.list_authorizations_by_role(role_id1, Some(auth_id1_1), None);
    assert_eq!(role1_auths_with_start_after.authorizations.len(), 1);
    assert_eq!(
        role1_auths_with_start_after.authorizations[0].id,
        auth_id1_2
    );

    let role2_auths = suite.list_authorizations_by_role(role_id2, None, None);
    assert_eq!(role2_auths.authorizations.len(), 1);
    assert_eq!(role2_auths.authorizations[0].id, auth_id2);

    let role3_auths = suite.list_authorizations_by_role(role_id3, None, None);
    assert_eq!(role3_auths.authorizations.len(), 1);
    assert_eq!(role3_auths.authorizations[0].id, auth_id3);

    // Test list assignments
    let assignments = suite.list_assignments(None, None);
    assert_eq!(assignments.assignments.len(), 3);

    let assignments_with_start_after = suite.list_assignments(
        Some((
            assignments.assignments[0].addr.clone(),
            assignments.assignments[0].role_id,
        )),
        None,
    );
    assert_eq!(assignments_with_start_after.assignments.len(), 2);

    let assignments_with_limit = suite.list_assignments(None, Some(1));
    assert_eq!(assignments_with_limit.assignments.len(), 1);

    // Test list addresses with role
    let addr_with_role1 = suite.list_addresses_with_role(role_id1, None, None);
    assert_eq!(addr_with_role1.addresses.len(), 2);
    assert_eq!(addr_with_role1.addresses[0].as_str(), ADDR0);
    assert_eq!(addr_with_role1.addresses[1].as_str(), ADDR1);

    let addr_with_role1_with_start_after =
        suite.list_addresses_with_role(role_id1, Some(ADDR0.to_string()), None);
    assert_eq!(addr_with_role1_with_start_after.addresses.len(), 1);
    assert_eq!(
        addr_with_role1_with_start_after.addresses[0].as_str(),
        ADDR1
    );

    let addr_with_role1_with_limit = suite.list_addresses_with_role(role_id1, None, Some(1));
    assert_eq!(addr_with_role1_with_limit.addresses.len(), 1);
    assert_eq!(addr_with_role1_with_limit.addresses[0].as_str(), ADDR0);

    // Test list roles for address
    let addr0_roles = suite.list_roles_for_address(ADDR0.to_string(), None, None);
    assert_eq!(addr0_roles.role_ids.len(), 2);
    assert_eq!(addr0_roles.role_ids[0], role_id1);
    assert_eq!(addr0_roles.role_ids[1], role_id2);

    let addr0_roles_with_start_after =
        suite.list_roles_for_address(ADDR0.to_string(), Some(role_id1), None);
    assert_eq!(addr0_roles_with_start_after.role_ids.len(), 1);
    assert_eq!(addr0_roles_with_start_after.role_ids[0], role_id2);

    let addr0_roles_with_limit = suite.list_roles_for_address(ADDR0.to_string(), None, Some(1));
    assert_eq!(addr0_roles_with_limit.role_ids.len(), 1);
    assert_eq!(addr0_roles_with_limit.role_ids[0], role_id1);

    let addr1_roles = suite.list_roles_for_address(ADDR1.to_string(), None, None);
    assert_eq!(addr1_roles.role_ids.len(), 1);
    assert_eq!(addr1_roles.role_ids[0], role_id1);
}

#[test]
fn test_pagination() {
    let mut suite = SuiteBuilder::base().build();
    let dao = suite.core_addr.clone();

    // Create many roles
    for i in 1..=15 {
        suite.create_role(&dao, format!("role{}", i), None, None, None, None);
    }

    // Test pagination with limit
    let first_page = suite.list_roles(None, Some(5));
    assert_eq!(first_page.roles.len(), 5);

    // Test pagination with start_after
    let second_page = suite.list_roles(Some(5), Some(5));
    assert_eq!(second_page.roles.len(), 5);
    assert_eq!(second_page.roles[0].id, 6);

    // Test last page
    let last_page = suite.list_roles(Some(10), Some(10));
    assert_eq!(last_page.roles.len(), 5); // Only 5 remaining
}

#[test]
fn test_complex_authorization_filters() {
    let mut suite = SuiteBuilder::base().build();
    let dao = suite.core_addr.clone();

    // Create a role
    let role_id = suite.create_role(&dao, "complex_role".to_string(), None, None, None, None);

    // Create authorization with complex filter (only allow execute messages to specific contract with specific method)
    let complex_filter = serde_json::json!({
        "wasm": {
            "execute": {
                "contract_addr": "target_contract",
                "msg": {
                    "#base64": {
                        "method": "allowed_method"
                    }
                }
            }
        }
    });

    suite.create_authorization(
        &dao,
        role_id,
        "complex_auth".to_string(),
        None,
        Some(complex_filter.clone()),
        Some(true),
        None,
    );

    suite.assign(
        &dao,
        vec![Assignment {
            addr: ADDR0.to_string(),
            role_id,
        }],
    );

    // Test that the complex filter works
    let allowed_msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: "target_contract".to_string(),
        msg: to_json_binary(&serde_json::json!({"method": "allowed_method"})).unwrap(),
        funds: vec![],
    });

    let disallowed_msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: "target_contract".to_string(),
        msg: to_json_binary(&serde_json::json!({"method": "disallowed_method"})).unwrap(),
        funds: vec![],
    });

    suite.assert_authorized(ADDR0, &allowed_msg);
    suite.assert_unauthorized(
        ADDR0,
        &disallowed_msg,
        Some(ContractError::NoMoreAuthorizations {}),
        None,
    );

    suite.assert_filter_passes(&complex_filter, &allowed_msg);
    suite.assert_filter_fails(
        &complex_filter,
        &disallowed_msg,
        Some(ContractError::MsgNotAllowedByFilter {
            err: cw_jsonfilter::FilterResult::operator_failed(
                "implicit equality check",
                "value does not match filter",
                "@.wasm.execute.msg.#base64.method",
                "@.wasm.execute.msg.method",
            )
            .as_fail()
            .unwrap()
            .to_string(),
        }),
    );
}

#[test]
fn test_role_with_initial_authorizations_and_assignments() {
    let initial_role = InitialRole {
        name: "multi_auth_role".to_string(),
        metadata: None,
        enabled: None, // default to true
        authorizations: Some(vec![
            InitialAuthorization {
                name: "auth1".to_string(),
                metadata: None,
                filter: Some(serde_json::json!({"type": "auth1"})),
                enabled: Some(true),
                skip_prepare: None,
            },
            InitialAuthorization {
                name: "auth2".to_string(),
                metadata: None,
                filter: Some(serde_json::json!({"type": "auth2"})),
                enabled: Some(false), // Disabled
                skip_prepare: None,
            },
        ]),
        assignments: Some(vec![ADDR0.to_string(), ADDR1.to_string()]),
    };

    let mut suite = SuiteBuilder::base().with_initial_role(initial_role).build();

    // Check that everything was created correctly
    suite.assert_role_count(1);
    suite.assert_authorization_count(2);
    suite.assert_assignment_count(2);

    // Check enabled
    suite.assert_role_enabled(1, true);
    suite.assert_authorization_enabled(2, true);
    suite.assert_authorization_enabled(3, false);

    // Check assignments
    suite.assert_assigned(ADDR0, 1, true);
    suite.assert_assigned(ADDR1, 1, true);
    suite.assert_assigned(ADDR2, 1, false);
}

#[test]
fn test_bulk_role_assignments() {
    let mut suite = SuiteBuilder::base().build();
    let dao = suite.core_addr.clone();

    // Create multiple roles
    let role_id1 = suite.create_role(&dao, "role1".to_string(), None, None, None, None);
    let role_id2 = suite.create_role(&dao, "role2".to_string(), None, None, None, None);
    let role_id3 = suite.create_role(&dao, "role3".to_string(), None, None, None, None);

    // Bulk assign multiple roles to multiple addresses
    suite.assign(
        &dao,
        vec![
            Assignment {
                addr: ADDR0.to_string(),
                role_id: role_id1,
            },
            Assignment {
                addr: ADDR0.to_string(),
                role_id: role_id2,
            },
            Assignment {
                addr: ADDR1.to_string(),
                role_id: role_id2,
            },
            Assignment {
                addr: ADDR1.to_string(),
                role_id: role_id3,
            },
        ],
    );

    // Verify all assignments
    suite.assert_assigned(ADDR0, role_id1, true);
    suite.assert_assigned(ADDR0, role_id2, true);
    suite.assert_assigned(ADDR0, role_id3, false);
    suite.assert_assigned(ADDR1, role_id1, false);
    suite.assert_assigned(ADDR1, role_id2, true);
    suite.assert_assigned(ADDR1, role_id3, true);
    suite.assert_assignment_count(4);

    // Bulk revoke some assignments
    suite.revoke(
        &dao,
        vec![
            Assignment {
                addr: ADDR0.to_string(),
                role_id: role_id1,
            },
            Assignment {
                addr: ADDR1.to_string(),
                role_id: role_id3,
            },
        ],
    );

    // Verify revocations
    suite.assert_assigned(ADDR0, role_id1, false);
    suite.assert_assigned(ADDR0, role_id2, true);
    suite.assert_assigned(ADDR1, role_id2, true);
    suite.assert_assigned(ADDR1, role_id3, false);
    suite.assert_assignment_count(2);
}

#[test]
fn test_authorization_filter_edge_cases() {
    let mut suite = SuiteBuilder::base().build();
    let dao = suite.core_addr.clone();

    // Create role
    let role_id = suite.create_role(&dao, "test_role".to_string(), None, None, None, None);

    // Test empty filter (should allow all)
    suite.create_authorization(
        &dao,
        role_id,
        "empty_filter".to_string(),
        None,
        Some(serde_json::json!({})),
        Some(true),
        None,
    );

    // Test null filter (should allow all)
    suite.create_authorization(
        &dao,
        role_id,
        "null_filter".to_string(),
        None,
        None,
        Some(true),
        None,
    );

    // Test very specific filter
    let specific_filter = serde_json::json!({
        "wasm": {
            "execute": {
                "contract_addr": "exact_contract",
                "msg": {
                    "exact_method": "exact_value",
                    "nested": {
                        "field": "exact_nested_value"
                    }
                },
                "funds": {
                    "$size": 0
                }
            }
        }
    });
    suite.create_authorization(
        &dao,
        role_id,
        "specific_filter".to_string(),
        None,
        Some(specific_filter),
        Some(true),
        None,
    );

    suite.assign(
        &dao,
        vec![Assignment {
            addr: ADDR0.to_string(),
            role_id,
        }],
    );

    // Test messages against different filters
    let any_msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: "any_contract".to_string(),
        msg: to_json_binary(&"any_msg").unwrap(),
        funds: vec![],
    });

    let specific_msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: "exact_contract".to_string(),
        msg: to_json_binary(&serde_json::json!({
            "exact_method": "exact_value",
            "nested": {
                "field": "exact_nested_value"
            }
        }))
        .unwrap(),
        funds: vec![],
    });

    // Any message should be authorized (empty and null filters allow all)
    suite.assert_authorized(ADDR0, &any_msg);

    // Specific message should be authorized by specific filter
    suite.assert_authorized(ADDR0, &specific_msg);
}

#[test]
fn test_multiple_roles_multiple_authorizations() {
    let mut suite = SuiteBuilder::base().build();
    let dao = suite.core_addr.clone();

    // Create two roles with different authorizations
    let role_id1 = suite.create_role(&dao, "role1".to_string(), None, None, None, None);
    let role_id2 = suite.create_role(&dao, "role2".to_string(), None, None, None, None);

    // Role 1: Can execute on contract A
    let filter_a = serde_json::json!({
        "wasm": {
            "execute": {
                "contract_addr": "contract_a"
            }
        }
    });
    suite.create_authorization(
        &dao,
        role_id1,
        "auth_a".to_string(),
        None,
        Some(filter_a),
        Some(true),
        None,
    );

    // Role 2: Can execute on contract B
    let filter_b = serde_json::json!({
        "wasm": {
            "execute": {
                "contract_addr": "contract_b"
            }
        }
    });
    suite.create_authorization(
        &dao,
        role_id2,
        "auth_b".to_string(),
        None,
        Some(filter_b),
        Some(true),
        None,
    );

    // ADDR0 gets both roles
    suite.assign(
        &dao,
        vec![
            Assignment {
                addr: ADDR0.to_string(),
                role_id: role_id1,
            },
            Assignment {
                addr: ADDR0.to_string(),
                role_id: role_id2,
            },
        ],
    );

    // ADDR1 gets only role 1
    suite.assign(
        &dao,
        vec![Assignment {
            addr: ADDR1.to_string(),
            role_id: role_id1,
        }],
    );

    // Test messages
    let msg_a = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: "contract_a".to_string(),
        msg: to_json_binary(&"test").unwrap(),
        funds: vec![],
    });

    let msg_b = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: "contract_b".to_string(),
        msg: to_json_binary(&"test").unwrap(),
        funds: vec![],
    });

    let msg_c = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: "contract_c".to_string(),
        msg: to_json_binary(&"test").unwrap(),
        funds: vec![],
    });

    // ADDR0 should be authorized for both A and B
    suite.assert_authorized(ADDR0, &msg_a);
    suite.assert_authorized(ADDR0, &msg_b);
    suite.assert_unauthorized(
        ADDR0,
        &msg_c,
        Some(ContractError::NoMoreAuthorizations {}),
        None,
    );

    // ADDR1 should only be authorized for A
    suite.assert_authorized(ADDR1, &msg_a);
    suite.assert_unauthorized(
        ADDR1,
        &msg_b,
        Some(ContractError::NoMoreAuthorizations {}),
        None,
    );
    suite.assert_unauthorized(
        ADDR1,
        &msg_c,
        Some(ContractError::NoMoreAuthorizations {}),
        None,
    );
}

#[test]
fn test_action_execution_with_multiple_actions() {
    let mut suite = SuiteBuilder::base().build();
    let dao = suite.core_addr.clone();

    // Set up role and authorization
    let role_id = suite.create_role(&dao, "executor".to_string(), None, None, None, None);
    let authorization_id = suite.create_authorization(
        &dao,
        role_id,
        "wasm_auth".to_string(),
        None,
        Some(serde_json::json!({
            "wasm": {
                "execute": {
                    "contract_addr": dao.to_string(),
                    "msg": {
                        "$exists": true,
                        "#base64": {
                            "update_config": {
                                "config": {
                                    "name": { "$contains": "new" }
                                }
                            }
                        }
                    },
                    "funds": []
                }
            }
        })),
        Some(true),
        None,
    );
    suite.assign(
        &dao,
        vec![Assignment {
            addr: ADDR0.to_string(),
            role_id,
        }],
    );

    // Create multiple actions
    let actions = vec![
        ActionToExecute {
            msg: CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: dao.to_string(),
                msg: to_json_binary(&dao_interface::msg::ExecuteMsg::UpdateConfig {
                    config: dao_interface::state::Config {
                        name: "new_name1".to_string(),
                        description: "new_description1".to_string(),
                        image_url: None,
                        automatically_add_cw20s: false,
                        automatically_add_cw721s: false,
                        dao_uri: None,
                    },
                })
                .unwrap(),
                funds: vec![],
            }),
            authorization_id,
        },
        ActionToExecute {
            msg: CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: dao.to_string(),
                msg: to_json_binary(&dao_interface::msg::ExecuteMsg::UpdateConfig {
                    config: dao_interface::state::Config {
                        name: "new_name2".to_string(),
                        description: "new_description2".to_string(),
                        image_url: None,
                        automatically_add_cw20s: false,
                        automatically_add_cw721s: false,
                        dao_uri: None,
                    },
                })
                .unwrap(),
                funds: vec![],
            }),
            authorization_id,
        },
        ActionToExecute {
            msg: CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: dao.to_string(),
                msg: to_json_binary(&dao_interface::msg::ExecuteMsg::UpdateConfig {
                    config: dao_interface::state::Config {
                        name: "new_name3".to_string(),
                        description: "new_description3".to_string(),
                        image_url: None,
                        automatically_add_cw20s: false,
                        automatically_add_cw721s: false,
                        dao_uri: None,
                    },
                })
                .unwrap(),
                funds: vec![],
            }),
            authorization_id,
        },
    ];

    // Execute all actions at once
    suite.execute_actions(ADDR0, actions);

    // Verify all actions were logged
    suite.assert_action_count(3);

    let mut logged_actions = suite.list_actions(None, None, None);
    assert_eq!(logged_actions.actions.len(), 3);

    // Verify they're all from ADDR0 with role 1 and auth 1
    for action in &logged_actions.actions {
        assert_eq!(action.addr.as_str(), ADDR0);
        assert_eq!(action.role_id, role_id);
        assert_eq!(action.authorization_id, authorization_id);
    }

    let reversed_actions = suite.list_actions(None, None, Some(true));
    assert_eq!(reversed_actions.actions.len(), 3);
    logged_actions.actions.reverse();
    assert_eq!(logged_actions.actions, reversed_actions.actions);

    // Verify the config has been updated from the the last action.
    let config = suite.base.get_config(&dao);
    assert_eq!(config.name, "new_name3");
    assert_eq!(config.description, "new_description3");

    // Attempt to execute action with invalid name
    let action = ActionToExecute {
        msg: CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: dao.to_string(),
            msg: to_json_binary(&dao_interface::msg::ExecuteMsg::UpdateConfig {
                config: dao_interface::state::Config {
                    name: "invalid_name".to_string(),
                    description: "new_description4".to_string(),
                    image_url: None,
                    automatically_add_cw20s: false,
                    automatically_add_cw721s: false,
                    dao_uri: None,
                },
            })
            .unwrap(),
            funds: vec![],
        }),
        authorization_id,
    };

    // Execute the action
    let err = suite.execute_actions_err(ADDR0, vec![action.clone()]);
    assert_eq!(
        err,
        ContractError::MsgNotAllowedByFilter {
            err: cw_jsonfilter::FilterResult::operator_failed(
                "$contains",
                "string value does not contain filter value",
                "@.wasm.execute.msg.#base64.update_config.config.name.$contains",
                "@.wasm.execute.msg.update_config.config.name",
            )
            .as_fail()
            .unwrap()
            .to_string(),
        }
    );

    // Check that the action was not executed
    let config = suite.base.get_config(&dao);
    assert_eq!(config.name, "new_name3");
    assert_eq!(config.description, "new_description3");

    // Update to an invalid filter.
    suite.update_authorization(
        &dao,
        authorization_id,
        None,
        OptionalUpdate(None),
        OptionalUpdate(Some(Update::Set(serde_json::json!({
            "$invalidOperator": {}
        })))),
        Some(true),
        None,
    );

    // Try to execute with invalid filter - should fail.
    let err = suite.execute_actions_err(ADDR0, vec![action.clone()]);
    assert_eq!(
        err,
        ContractError::FilterError {
            err: cw_jsonfilter::FilterResult::fatal_unknown_operator(
                "$invalidOperator",
                "@.$invalidOperator",
                "@",
            )
            .as_fatal()
            .unwrap()
            .to_string(),
        }
    );

    // Set filter contract to invalid address.
    suite.update_filter(
        &dao,
        ModuleUpdate::Existing {
            address: "cosmwasm1x0tyu0n84rz4dv2026zrlfa6gnhk5mfcv72qeu4fqvpmjczgje8q3p927d".to_string(),
        },
    );

    // Try to execute with invalid filter - should fail.
    let err = suite.execute_actions_err(ADDR0, vec![action]);
    assert!(matches!(
        err,
        ContractError::FilterContractQueryError { .. }
    ));
}

#[test]
fn test_comprehensive_list_queries_with_filtering() {
    let mut suite = SuiteBuilder::base().build();
    let dao = suite.core_addr.clone();

    // Give DAO tokens to spend.
    suite.base.mint(&dao, 100u128, DENOM);

    // Create complex setup
    let role_id1 = suite.create_role(&dao, "role1".to_string(), None, None, None, None);
    let role_id2 = suite.create_role(&dao, "role2".to_string(), None, None, None, None);

    // Multiple authorizations per role
    let authorization_id1 = suite.create_authorization(
        &dao,
        role_id1,
        "role1_auth1".to_string(),
        None,
        Some(serde_json::json!({})), // Allow all
        Some(true),
        None,
    );
    suite.create_authorization(
        &dao,
        role_id1,
        "role1_auth2".to_string(),
        None,
        Some(serde_json::json!({})), // Allow all
        Some(true),
        None,
    );
    let authorization_id3 = suite.create_authorization(
        &dao,
        role_id2,
        "role2_auth1".to_string(),
        None,
        Some(serde_json::json!({})), // Allow all
        Some(true),
        None,
    );

    // Complex assignments
    suite.assign(
        &dao,
        vec![
            Assignment {
                addr: ADDR0.to_string(),
                role_id: role_id1,
            },
            Assignment {
                addr: ADDR0.to_string(),
                role_id: role_id2,
            },
            Assignment {
                addr: ADDR1.to_string(),
                role_id: role_id1,
            },
        ],
    );

    // Execute some actions to test action queries
    let action1 = ActionToExecute {
        msg: CosmosMsg::Bank(BankMsg::Send {
            to_address: ADDR1.to_string(),
            amount: vec![Coin {
                denom: DENOM.to_string(),
                amount: Uint128::from(50u128),
            }],
        }),
        authorization_id: authorization_id1,
    };
    let action2 = ActionToExecute {
        msg: CosmosMsg::Bank(BankMsg::Send {
            to_address: ADDR1.to_string(),
            amount: vec![Coin {
                denom: DENOM.to_string(),
                amount: Uint128::from(25u128),
            }],
        }),
        authorization_id: authorization_id3,
    };
    let action3 = ActionToExecute {
        msg: CosmosMsg::Bank(BankMsg::Send {
            to_address: ADDR1.to_string(),
            amount: vec![Coin {
                denom: DENOM.to_string(),
                amount: Uint128::from(1u128),
            }],
        }),
        authorization_id: authorization_id3,
    };

    suite.execute_actions(ADDR0, vec![action1]);
    suite.execute_actions(ADDR0, vec![action2, action3]);

    // Test list actions by role
    let role1_actions = suite.list_actions_by_role(role_id1, None, None, None);
    assert_eq!(role1_actions.actions.len(), 1);

    let mut role2_actions = suite.list_actions_by_role(role_id2, None, None, None);
    assert_eq!(role2_actions.actions.len(), 2);

    let role2_actions_start_after =
        suite.list_actions_by_role(role_id2, Some(role2_actions.actions[0].id), None, None);
    assert_eq!(role2_actions_start_after.actions.len(), 1);
    assert_eq!(
        role2_actions_start_after.actions[0].id,
        role2_actions.actions[1].id
    );

    let role2_actions_reversed = suite.list_actions_by_role(role_id2, None, None, Some(true));
    assert_eq!(role2_actions_reversed.actions.len(), 2);
    role2_actions.actions.reverse();
    assert_eq!(role2_actions_reversed.actions, role2_actions.actions);

    // Test list actions by authorization
    let auth1_actions = suite.list_actions_by_authorization(authorization_id1, None, None, None);
    assert_eq!(auth1_actions.actions.len(), 1);

    let mut auth3_actions =
        suite.list_actions_by_authorization(authorization_id3, None, None, None);
    assert_eq!(auth3_actions.actions.len(), 2);

    let auth3_actions_start_after = suite.list_actions_by_authorization(
        authorization_id3,
        Some(auth3_actions.actions[0].id),
        None,
        None,
    );
    assert_eq!(auth3_actions_start_after.actions.len(), 1);
    assert_eq!(
        auth3_actions_start_after.actions[0].id,
        auth3_actions.actions[1].id
    );

    let auth3_actions_reversed =
        suite.list_actions_by_authorization(authorization_id3, None, None, Some(true));
    assert_eq!(auth3_actions_reversed.actions.len(), 2);
    auth3_actions.actions.reverse();
    assert_eq!(auth3_actions_reversed.actions, auth3_actions.actions);

    // Test list actions by address
    let mut addr0_actions = suite.list_actions_by_address(ADDR0.to_string(), None, None, None);
    assert_eq!(addr0_actions.actions.len(), 3);

    let addr0_actions_start_after = suite.list_actions_by_address(
        ADDR0.to_string(),
        Some(addr0_actions.actions[0].id),
        None,
        None,
    );
    assert_eq!(addr0_actions_start_after.actions.len(), 2);
    assert_eq!(
        addr0_actions_start_after.actions[0].id,
        addr0_actions.actions[1].id
    );

    let addr0_reversed_actions =
        suite.list_actions_by_address(ADDR0.to_string(), None, None, Some(true));
    assert_eq!(addr0_reversed_actions.actions.len(), 3);
    addr0_actions.actions.reverse();
    assert_eq!(addr0_reversed_actions.actions, addr0_actions.actions);

    let addr1_actions = suite.list_actions_by_address(ADDR1.to_string(), None, None, None);
    assert_eq!(addr1_actions.actions.len(), 0);
}

#[test]
fn test_edge_case_empty_operations() {
    let mut suite = SuiteBuilder::base().build();
    let dao = suite.core_addr.clone();

    // Test empty assign/revoke operations
    let err = suite.assign_err(&dao, vec![]);
    assert_eq!(err, ContractError::NoRoles {});

    let err = suite.revoke_err(&dao, vec![]);
    assert_eq!(err, ContractError::NoRoles {});

    // Test empty action execution
    let err = suite.execute_actions_err(ADDR0, vec![]);
    assert_eq!(err, ContractError::NoActions {});

    // Nothing should have changed
    suite.assert_assignment_count(0);
    suite.assert_action_count(0);
}

#[test]
fn test_role_and_authorization_metadata_updates() {
    let mut suite = SuiteBuilder::base().build();
    let dao = suite.core_addr.clone();

    // Create role with metadata
    let role_id = suite.create_role(
        &dao,
        "test_role".to_string(),
        Some("Initial metadata".to_string()),
        None,
        None,
        None,
    );

    // Create authorization with metadata
    let authorization_id = suite.create_authorization(
        &dao,
        role_id,
        "test_auth".to_string(),
        Some("Initial auth metadata".to_string()),
        None,
        None,
        None,
    );

    // Test metadata updates using OptionalUpdate variants

    // Update role metadata to new value
    suite.update_role(
        &dao,
        role_id,
        None,
        OptionalUpdate(Some(Update::Set("Updated metadata".to_string()))),
        None,
    );

    // Update authorization metadata to clear it
    suite.update_authorization(
        &dao,
        authorization_id,
        None,
        OptionalUpdate(Some(Update::Clear)),
        OptionalUpdate(None),
        None,
        None,
    );

    // Verify updates by checking the stored values
    let role = suite.get_role(role_id);
    assert_eq!(role.metadata, Some("Updated metadata".to_string()));

    let auth = suite.get_authorization(authorization_id);
    assert_eq!(auth.authorization.metadata, None);
}

#[test]
fn test_protobuf_filter() {
    let mut suite = SuiteBuilder::base().build();
    let dao = suite.core_addr.clone();

    // Create a role with an authorization that allows a true BoolValue.
    let role_id = suite.create_role(
        &dao,
        "test_role".to_string(),
        None,
        None,
        None,
        Some(vec![ADDR0.to_string()]),
    );

    // Error if no messages are registered.
    let err = suite.create_authorization_err(
        &dao,
        role_id,
        "test_auth".to_string(),
        None,
        Some(serde_json::json!({"#proto": {"type": "google.protobuf.BoolValue", "value": true}})),
        None,
    );
    assert!(matches!(
        err,
        ContractError::ProtobufRegistryPrepareFailed { .. }
    ));
    assert!(err.to_string().contains(
        &cw_protobuf_registry::ContractError::MessageNotFound {
            message: "google.protobuf.BoolValue".to_string(),
        }
        .to_string()
    ));

    // Register the protobuf file descriptor set.
    suite.register_protobufs(&dao, vec![suite.google_string_bool_fds.clone()]);

    // Successfully create an authorization that allows a true BoolValue.
    let authorization_id = suite.create_authorization(
        &dao,
        role_id,
        "test_auth".to_string(),
        None,
        Some(serde_json::json!({"#proto": {"type": "google.protobuf.BoolValue", "value": true}})),
        Some(true),
        None,
    );

    // Update the authorization to include a StringValue.
    suite.update_authorization(
        &dao,
        authorization_id,
        None,
        OptionalUpdate(None),
        OptionalUpdate(Some(Update::Set(
            serde_json::json!({"#proto": {"type": "google.protobuf.StringValue", "value": "test"}}),
        ))),
        None,
        None,
    );

    // Test that the protobuf filter works.
    suite.update_authorization(
        &dao,
        authorization_id,
        None,
        OptionalUpdate(None),
        OptionalUpdate(Some(Update::Set(
            serde_json::json!({"stargate": {"type_url": "google.protobuf.StringValue", "value": {"#proto": {"type": "google.protobuf.StringValue", "value": "pass"}}}}),
        ))),
        None,
        None,
    );

    let pool = DescriptorPool::from_file_descriptor_set(
        FileDescriptorSet::decode(suite.google_string_bool_fds.as_slice()).unwrap(),
    )
    .unwrap();

    let base64_encoded_pass = base64_encode_protobuf(
        &pool,
        "google.protobuf.StringValue",
        &serde_json::json!("pass"),
    );
    suite.assert_authorized_by(
        ADDR0,
        authorization_id,
        &CosmosMsg::Stargate {
            type_url: "google.protobuf.StringValue".to_string(),
            value: Binary::from_base64(&base64_encoded_pass).unwrap(),
        },
    );

    let base64_encoded_not_pass = base64_encode_protobuf(
        &pool,
        "google.protobuf.StringValue",
        &serde_json::json!("not_pass"),
    );
    suite.assert_unauthorized_by(
        ADDR0,
        authorization_id,
        &CosmosMsg::Stargate {
            type_url: "google.protobuf.StringValue".to_string(),
            value: Binary::from_base64(&base64_encoded_not_pass).unwrap(),
        },
        Some(ContractError::MsgNotAllowedByFilter {
            err: cw_jsonfilter::FilterResult::operator_failed(
                "implicit equality check",
                "value does not match filter",
                "@.stargate.value.#proto",
                "@.stargate.value",
            )
            .as_fail()
            .unwrap()
            .to_string(),
        }),
    );
}
