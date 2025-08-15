use cosmwasm_std::{coins, to_json_binary, BankMsg, CosmosMsg};
use cw_filter::ContractError;
use cw_ownable::OwnershipError;
use dao_interface::state::{ModuleInstantiateInfo, ModuleUpdate};
use dao_testing::OWNER;
use serde_json::json;

use crate::{
    msg::{FilterResponse, InstantiateMsg},
    testing::suite::SuiteBuilder,
};

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
    assert_eq!(info.info.contract, "crates.io:cw-filter");
    assert_eq!(info.info.version, env!("CARGO_PKG_VERSION"));
}

#[test]
fn test_init_owner() {
    let mut suite = SuiteBuilder::base().build();
    let other_owner = "other_owner";

    suite.filter_addr = suite.base.instantiate(
        suite.base.filter_id,
        OWNER,
        &InstantiateMsg {
            owner: Some(other_owner.to_string()),
            protobuf_registry: Some(ModuleUpdate::Existing {
                address: suite.protobuf_registry_addr.to_string(),
            }),
        },
        &[],
        "new filter",
        None,
    );

    let owner = suite.get_ownership().owner.unwrap();
    assert_eq!(owner, other_owner);

    let protobuf_registry_addr = suite.get_protobuf_registry();
    assert_eq!(protobuf_registry_addr, Some(suite.protobuf_registry_addr));
}

#[test]
fn test_no_protobuf_registry() {
    let mut suite = SuiteBuilder::base().build();

    suite.filter_addr = suite.base.instantiate(
        suite.base.filter_id,
        OWNER,
        &InstantiateMsg {
            owner: None,
            protobuf_registry: None,
        },
        &[],
        "new filter",
        None,
    );

    let protobuf_registry_addr = suite.get_protobuf_registry();
    assert_eq!(protobuf_registry_addr, None);

    // filter with #stargate fails
    suite.assert_filter(
        json!({
            "#stargate": {
                "type_url": "/does.not.exist",
                "value": "test"
            }
        }),
        CosmosMsg::Stargate {
            type_url: "/does.not.exist".to_string(),
            value: to_json_binary(&json!({
                "test": "value"
            }))
            .unwrap(),
        },
        FilterResponse::Fatal {
            reason: ContractError::MissingProtobufRegistry {}.to_string(),
        },
    );
}

#[test]
fn test_update_protobuf_registry() {
    let mut suite = SuiteBuilder::base().build();

    // only the owner can update the protobuf registry
    let err = suite.update_protobuf_registry_err("not_owner", None);
    assert_eq!(err, ContractError::Ownership(OwnershipError::NotOwner {}));

    suite.assert_protobuf_registry(Some(suite.protobuf_registry_addr.clone()));

    suite.update_protobuf_registry(None);
    suite.assert_protobuf_registry(None);

    suite.update_protobuf_registry(Some(ModuleUpdate::New(ModuleInstantiateInfo {
        code_id: suite.base.protobuf_registry_id,
        msg: to_json_binary(&cw_protobuf_registry::msg::InstantiateMsg {
            owner: Some(OWNER.to_string()),
        })
        .unwrap(),
        admin: Some(dao_interface::state::Admin::CoreModule {}),
        funds: None,
        label: "new_protobuf_registry".to_string(),
        salt: None,
    })));
    let new_protobuf_registry_addr = suite.get_protobuf_registry();
    assert!(new_protobuf_registry_addr.is_some());
    assert_ne!(
        new_protobuf_registry_addr,
        Some(suite.protobuf_registry_addr.clone())
    );

    suite.update_protobuf_registry(Some(ModuleUpdate::Existing {
        address: suite.protobuf_registry_addr.to_string(),
    }));
    suite.assert_protobuf_registry(Some(suite.protobuf_registry_addr.clone()));
}

#[test]
fn test_filter() {
    let mut suite = SuiteBuilder::base().build();

    suite.assert_filter(
        json!({
            "bank": {
                "send": {
                    "to_address": "recipient",
                    "amount": [
                        {
                            "denom": "ucosm",
                            "amount": "100"
                        }
                    ]
                }
            }
        }),
        CosmosMsg::Bank(BankMsg::Send {
            to_address: "recipient".to_string(),
            amount: coins(100, "ucosm"),
        }),
        FilterResponse::Pass {},
    );

    suite.assert_filter(
        json!({
            "bank": {
                "send": {
                    "to_address": "invalid_address",
                    "amount": [
                        {
                            "denom": "ucosm",
                            "amount": "100"
                        }
                    ]
                }
            }
        }),
        CosmosMsg::Bank(BankMsg::Send {
            to_address: "recipient".to_string(),
            amount: coins(100, "ucosm"),
        }),
        FilterResponse::Fail {
            reason: "Operator failed: `implicit equality check` at filter path: `@.bank.send.to_address` and object path: `@.bank.send.to_address` with reason: `value does not match filter`".to_string(),
        },
    );

    suite.assert_filter(
        json!({
            "bank": {
                "send": {
                    "to_address": "recipient",
                    "amount": [
                        {
                            "denom": "ucosm",
                            "amount": "200"
                        }
                    ]
                }
            }
        }),
        CosmosMsg::Bank(BankMsg::Send {
            to_address: "recipient".to_string(),
            amount: coins(100, "ucosm"),
        }),
        FilterResponse::Fail {
            reason: "Operator failed: `implicit equality check` at filter path: `@.bank.send.amount[0].amount` and object path: `@.bank.send.amount[0].amount` with reason: `value does not match filter`".to_string(),
        },
    );

    suite.assert_filter(
        json!({
            "bank": {
                "send": {
                    "amount": [
                        {
                            "amount": {
                                "$between": [10, 5]
                            }
                        }
                    ]
                }
            }
        }),
        CosmosMsg::Bank(BankMsg::Send {
            to_address: "recipient".to_string(),
            amount: coins(7, "ucosm"),
        }),
        FilterResponse::Fatal {
            reason: "Invalid filter: `$between args must be in ascending order` at filter path: `@.bank.send.amount[0].amount.$between` and object path: `@.bank.send.amount[0].amount`".to_string(),
        },
    );
}
