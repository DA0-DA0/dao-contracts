use cosmwasm_std::StdError;
use cw_ownable::OwnershipError;
use cw_protobuf_registry::ContractError;
use dao_testing::OWNER;
use prost::Message;
use prost_reflect::DescriptorPool;
use prost_types::FileDescriptorSet;

use crate::{msg::InstantiateMsg, protobuf::encode_protobuf, testing::suite::SuiteBuilder};

#[test]
fn test_init_with_owner() {
    let mut suite = SuiteBuilder::base().build();
    let other_owner = "other_owner";

    suite.registry_addr = suite.base.instantiate(
        suite.base.protobuf_registry_id,
        OWNER,
        &InstantiateMsg {
            owner: Some(other_owner.to_string()),
        },
        &[],
        "protobuf-registry",
        None,
    );

    let owner = suite.get_ownership().owner.unwrap();
    assert_eq!(owner.as_str(), other_owner);
}

#[test]
fn test_update_owner() {
    let mut suite = SuiteBuilder::base().build();

    let existing_owner = suite.get_ownership().owner.unwrap();
    assert_eq!(existing_owner.as_str(), OWNER);

    let new_owner = "cosmwasm1lk0ans8sykcdtc2u6ep502pjm6m2ep4aqe9qsupg5hwpweg4mxxqrsvg0k";
    suite.update_owner(existing_owner, new_owner);

    let owner = suite.get_ownership().owner.unwrap();
    assert_eq!(owner.as_str(), new_owner);
}

#[test]
fn test_info() {
    let mut suite = SuiteBuilder::base().build();
    let info = suite.get_info();
    assert_eq!(info.info.contract, "crates.io:cw-protobuf-registry");
    assert_eq!(info.info.version, env!("CARGO_PKG_VERSION"));
}

#[test]
fn test_auth() {
    let mut suite = SuiteBuilder::base().build();
    let not_owner = "cosmwasm1hclhm4dapgs8lxc9ya59jjyakln279wc0rh6ewx5ddrmhf0jlctq3mddc2";

    // only the owner can register
    let err = suite.register_err(not_owner, vec![]);
    assert_eq!(err, ContractError::Ownership(OwnershipError::NotOwner {}));

    // only the owner can unregister
    let err = suite.unregister_err(not_owner, vec![], None);
    assert_eq!(err, ContractError::Ownership(OwnershipError::NotOwner {}));

    // only the owner can prepare
    let err = suite.prepare_err(not_owner, vec![]);
    assert_eq!(err, ContractError::Ownership(OwnershipError::NotOwner {}));

    // only the owner can unprepare
    let err = suite.unprepare_err(not_owner, vec![]);
    assert_eq!(err, ContractError::Ownership(OwnershipError::NotOwner {}));
}

#[test]
fn test_protobuf_management() {
    let mut suite = SuiteBuilder::base().build();

    let err = suite.register_err(OWNER, vec![]);
    assert_eq!(err, ContractError::NoFiles {});

    let err = suite.unregister_err(OWNER, vec![], None);
    assert_eq!(err, ContractError::NoFiles {});

    // Create a protobuf file descriptor set
    let crate_root = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let proto_path = std::path::Path::new(&crate_root).join("proto/string_bool_value.pb");
    let file_descriptor_set = std::fs::read(proto_path).unwrap();

    suite.register(OWNER, vec![file_descriptor_set.clone()]);
    // Registering the same file descriptor set again should be a no-op.
    suite.register(OWNER, vec![file_descriptor_set]);

    suite.assert_files(vec!["google/protobuf/wrappers.proto".to_string()]);
    suite.assert_messages(vec![
        "google.protobuf.BoolValue".to_string(),
        "google.protobuf.StringValue".to_string(),
    ]);
    suite.assert_messages_by_file(
        "google/protobuf/wrappers.proto",
        vec![
            "google.protobuf.BoolValue".to_string(),
            "google.protobuf.StringValue".to_string(),
        ],
    );

    // No messages are prepared.
    suite.assert_list_prepared(vec![]);
    suite.assert_prepared("google.protobuf.BoolValue", false);
    suite.assert_prepared("google.protobuf.StringValue", false);

    suite.prepare(OWNER, vec!["google.protobuf.BoolValue".to_string()]);
    suite.assert_list_prepared(vec!["google.protobuf.BoolValue".to_string()]);
    suite.assert_prepared("google.protobuf.BoolValue", true);
    suite.assert_prepared("google.protobuf.StringValue", false);

    suite.prepare(OWNER, vec!["google.protobuf.StringValue".to_string()]);
    suite.assert_list_prepared(vec![
        "google.protobuf.BoolValue".to_string(),
        "google.protobuf.StringValue".to_string(),
    ]);
    suite.assert_prepared("google.protobuf.BoolValue", true);
    suite.assert_prepared("google.protobuf.StringValue", true);

    suite.unprepare(
        OWNER,
        vec![
            "google.protobuf.BoolValue".to_string(),
            "google.protobuf.StringValue".to_string(),
        ],
    );
    suite.assert_list_prepared(vec![]);
    suite.assert_prepared("google.protobuf.BoolValue", false);
    suite.assert_prepared("google.protobuf.StringValue", false);

    // Errors when partially unregistering messages from multiple files and
    // doesn't reach the final file name before the limit is reached.
    let err = suite.unregister_err(
        OWNER,
        vec!["google/protobuf/wrappers.proto".to_string(), "".to_string()],
        Some(1),
    );
    assert_eq!(
        err,
        ContractError::MessageLimitReached {
            unregistered: 1,
            total: 2,
        }
    );

    // Allows partial unregistering of messages from a single file.
    suite.unregister(
        OWNER,
        vec!["google/protobuf/wrappers.proto".to_string()],
        Some(1),
    );

    // File removed, but some messages remain.
    suite.assert_files(vec![]);
    suite.assert_messages(vec!["google.protobuf.StringValue".to_string()]);

    // Finishing unregistering the file removes all messages.
    suite.unregister(
        OWNER,
        vec!["google/protobuf/wrappers.proto".to_string()],
        None,
    );
    suite.assert_messages(vec![]);
}

#[test]
fn test_regen_protobuf_filter() {
    let mut suite = SuiteBuilder::base().build();

    // Attempt to get the file descriptor set for a message that doesn't exist.
    let err =
        suite.file_descriptor_set_err(vec!["regen.ecocredit.basket.v1.MsgCreate".to_string()]);
    assert_eq!(
        err,
        StdError::generic_err(format!(
            "Querier contract error: {}",
            StdError::generic_err(
                ContractError::MessageNotFound {
                    message: "regen.ecocredit.basket.v1.MsgCreate".to_string(),
                }
                .to_string()
            )
        ))
    );

    // Register the protobuf file descriptor set.

    let crate_root = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let proto_path = std::path::Path::new(&crate_root).join("proto/regen_ecocredit.pb");
    let file_descriptor_set = std::fs::read(proto_path).unwrap();

    // Register the protobuf file descriptor set.
    suite.register(OWNER, vec![file_descriptor_set.clone()]);

    // Get the file descriptor set for a message, and ensure it contains exactly
    // the files and messages necessary and nothing more.
    let fds = suite.file_descriptor_set(vec!["regen.ecocredit.basket.v1.MsgCreate".to_string()]);
    assert_eq!(fds.file.len(), 5);

    assert_eq!(
        fds.file[0].name.as_ref().unwrap(),
        "google/protobuf/timestamp.proto"
    );
    assert_eq!(fds.file[0].message_type.len(), 1);
    assert_eq!(
        fds.file[0].message_type[0].name.as_ref().unwrap(),
        "Timestamp"
    );

    assert_eq!(
        fds.file[1].name.as_ref().unwrap(),
        "google/protobuf/duration.proto"
    );
    assert_eq!(fds.file[1].message_type.len(), 1);
    assert_eq!(
        fds.file[1].message_type[0].name.as_ref().unwrap(),
        "Duration"
    );

    assert_eq!(
        fds.file[2].name.as_ref().unwrap(),
        "regen/ecocredit/basket/v1/types.proto"
    );
    assert_eq!(fds.file[2].message_type.len(), 1);
    assert_eq!(
        fds.file[2].message_type[0].name.as_ref().unwrap(),
        "DateCriteria"
    );

    assert_eq!(
        fds.file[3].name.as_ref().unwrap(),
        "cosmos/base/v1beta1/coin.proto"
    );
    assert_eq!(fds.file[3].message_type.len(), 1);
    assert_eq!(fds.file[3].message_type[0].name.as_ref().unwrap(), "Coin");

    assert_eq!(
        fds.file[4].name.as_ref().unwrap(),
        "regen/ecocredit/basket/v1/tx.proto"
    );
    assert_eq!(fds.file[4].message_type.len(), 1);
    assert_eq!(
        fds.file[4].message_type[0].name.as_ref().unwrap(),
        "MsgCreate"
    );
}

#[test]
fn test_prepare_and_decode() {
    let mut suite = SuiteBuilder::base().build();

    // Register the protobuf file descriptor set.

    let crate_root = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let proto_path = std::path::Path::new(&crate_root).join("proto/regen_ecocredit.pb");
    let file_descriptor_set = std::fs::read(proto_path).unwrap();

    let pool = DescriptorPool::from_file_descriptor_set(
        FileDescriptorSet::decode(file_descriptor_set.as_slice()).unwrap(),
    )
    .unwrap();

    let encoded_coin = encode_protobuf(
        &pool,
        "cosmos.base.v1beta1.Coin",
        &serde_json::json!({
            "amount": "123",
            "denom": "abc"
        }),
    );

    // not yet registered
    let err = suite.decode_err("cosmos.base.v1beta1.Coin", encoded_coin.clone());
    assert_eq!(
        err,
        StdError::generic_err(format!(
            "Querier contract error: {}",
            StdError::generic_err(format!(
                "failed to create file descriptor set: {}",
                ContractError::MessageNotFound {
                    message: "cosmos.base.v1beta1.Coin".to_string()
                }
            ))
        ))
    );

    // Register the protobuf file descriptor set.
    suite.register(OWNER, vec![file_descriptor_set.clone()]);

    // decode works when registered even if not prepared (less efficient)
    suite.assert_decode(
        "cosmos.base.v1beta1.Coin",
        encoded_coin.clone(),
        serde_json::json!({"amount": "123", "denom": "abc"}),
    );

    // file descriptor set works when unprepared
    let fds = suite.file_descriptor_set(vec!["cosmos.base.v1beta1.Coin".to_string()]);
    assert_eq!(fds.file.len(), 1);
    assert_eq!(
        fds.file[0].name.as_ref().unwrap(),
        "cosmos/base/v1beta1/coin.proto"
    );
    assert_eq!(fds.file[0].message_type.len(), 1);
    assert_eq!(fds.file[0].message_type[0].name.as_ref().unwrap(), "Coin");

    // prepare so future decodes are faster
    suite.prepare(
        OWNER,
        vec![
            "regen.ecocredit.basket.v1.MsgCreate".to_string(),
            "cosmos.base.v1beta1.Coin".to_string(),
        ],
    );

    suite.assert_list_prepared(vec![
        "cosmos.base.v1beta1.Coin".to_string(),
        "regen.ecocredit.basket.v1.MsgCreate".to_string(),
    ]);
    suite.assert_prepared("regen.ecocredit.basket.v1.MsgCreate", true);
    suite.assert_prepared("regen.ecocredit.basket.v1.DateCriteria", false);
    suite.assert_prepared("cosmos.base.v1beta1.Coin", true);
    suite.assert_prepared("google.protobuf.Duration", false);
    suite.assert_prepared("google.protobuf.Timestamp", false);

    // decode works when prepared
    suite.assert_decode(
        "cosmos.base.v1beta1.Coin",
        encoded_coin.clone(),
        serde_json::json!({"amount": "123", "denom": "abc"}),
    );

    // file descriptor set works when prepared
    let fds = suite.file_descriptor_set(vec!["cosmos.base.v1beta1.Coin".to_string()]);
    assert_eq!(fds.file.len(), 1);
    assert_eq!(
        fds.file[0].name.as_ref().unwrap(),
        "cosmos/base/v1beta1/coin.proto"
    );
    assert_eq!(fds.file[0].message_type.len(), 1);
    assert_eq!(fds.file[0].message_type[0].name.as_ref().unwrap(), "Coin");

    let err = suite.decode_err("wrong_message", encoded_coin.clone());
    assert_eq!(
        err,
        StdError::generic_err(format!(
            "Querier contract error: {}",
            StdError::generic_err(format!(
                "failed to create file descriptor set: {}",
                ContractError::MessageNotFound {
                    message: "wrong_message".to_string()
                }
            ))
        ))
    );

    let err = suite.decode_err("cosmos.base.v1beta1.Coin", vec![0x1, 0x2, 0x3]);
    assert_eq!(
        err,
        StdError::generic_err(format!(
            "Querier contract error: {}",
            StdError::generic_err("failed to decode Protobuf message: invalid tag value: 0")
        ))
    );

    suite.assert_decode(
        "cosmos.base.v1beta1.Coin",
        encoded_coin.clone(),
        serde_json::json!({"amount": "123", "denom": "abc"}),
    );

    // unprepare so future decodes are less efficient
    suite.unprepare(OWNER, vec!["cosmos.base.v1beta1.Coin".to_string()]);

    // works when unprepared
    suite.assert_decode(
        "cosmos.base.v1beta1.Coin",
        encoded_coin,
        serde_json::json!({"amount": "123", "denom": "abc"}),
    );
}
