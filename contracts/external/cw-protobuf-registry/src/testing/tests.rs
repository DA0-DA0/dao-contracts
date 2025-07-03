use cosmwasm_std::StdError;
use cw_protobuf_registry::ContractError;
use dao_testing::OWNER;

use crate::testing::suite::SuiteBuilder;

#[test]
fn test_instantiate() {
    SuiteBuilder::base().build();
}

#[test]
fn test_update_owner() {
    let mut suite = SuiteBuilder::base().build();

    let existing_owner = suite.get_ownership().owner.unwrap();
    assert_eq!(existing_owner, OWNER);

    let new_owner = "new_owner";
    suite.update_owner(existing_owner, new_owner);

    let owner = suite.get_ownership().owner.unwrap();
    assert_eq!(owner, new_owner);
}

#[test]
fn test_info() {
    let mut suite = SuiteBuilder::base().build();
    let info = suite.get_info();
    assert_eq!(info.info.contract, "crates.io:cw-protobuf-registry");
    assert_eq!(info.info.version, env!("CARGO_PKG_VERSION"));
}

#[test]
fn test_protobuf_management() {
    let mut suite = SuiteBuilder::base().build();

    // Create a protobuf file descriptor set
    let crate_root = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let proto_path = std::path::Path::new(&crate_root).join("proto/string_bool_value.pb");
    let file_descriptor_set = std::fs::read(proto_path).unwrap();

    suite.register_protobufs(OWNER, vec![file_descriptor_set]);

    suite.assert_protobuf_files(vec!["google/protobuf/wrappers.proto".to_string()]);
    suite.assert_protobuf_messages(vec![
        "google.protobuf.BoolValue".to_string(),
        "google.protobuf.StringValue".to_string(),
    ]);
    suite.assert_protobuf_messages_by_file(
        "google/protobuf/wrappers.proto",
        vec![
            "google.protobuf.BoolValue".to_string(),
            "google.protobuf.StringValue".to_string(),
        ],
    );

    // Errors when partially unregistering messages from multiple files and
    // doesn't reach the final file name before the limit is reached.
    let err = suite.unregister_protobufs_err(
        OWNER,
        vec!["google/protobuf/wrappers.proto".to_string(), "".to_string()],
        Some(1),
    );
    assert_eq!(
        err,
        ContractError::ProtobufMessageLimitReached {
            unregistered: 1,
            total: 2,
        }
    );

    // Allows partial unregistering of messages from a single file.
    suite.unregister_protobufs(
        OWNER,
        vec!["google/protobuf/wrappers.proto".to_string()],
        Some(1),
    );

    // File removed, but some messages remain.
    suite.assert_protobuf_files(vec![]);
    suite.assert_protobuf_messages(vec!["google.protobuf.StringValue".to_string()]);

    // Finishing unregistering the file removes all messages.
    suite.unregister_protobufs(
        OWNER,
        vec!["google/protobuf/wrappers.proto".to_string()],
        None,
    );
    suite.assert_protobuf_messages(vec![]);
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
                ContractError::ProtobufMessageNotFound {
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
    suite.register_protobufs(OWNER, vec![file_descriptor_set.clone()]);

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
