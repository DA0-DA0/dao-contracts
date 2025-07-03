use cosmwasm_std::{Binary, CosmosMsg, StdError};
use cw_jsonfilter::base64_encode_protobuf;
use dao_testing::ADDR0;
use osmosis_std_derive::CosmwasmExt;
use prost::Message;
use prost_reflect::DescriptorPool;
use prost_types::FileDescriptorSet;

use crate::testing::suite::SuiteBuilder;
use dao_rbam::ContractError;

/// Coin defines a token with a denomination and an amount.
///
/// NOTE: The amount field is an Int which implements the custom method
/// signatures required by gogoproto.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(
    Clone,
    PartialEq,
    Eq,
    ::prost::Message,
    ::serde::Serialize,
    ::serde::Deserialize,
    ::schemars::JsonSchema,
    CosmwasmExt,
)]
#[proto_message(type_url = "/cosmos.base.v1beta1.Coin")]
pub struct Coin {
    #[prost(string, tag = "1")]
    pub denom: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub amount: ::prost::alloc::string::String,
}

#[derive(
    Clone,
    PartialEq,
    Eq,
    ::prost::Message,
    ::serde::Serialize,
    ::serde::Deserialize,
    ::schemars::JsonSchema,
    CosmwasmExt,
)]
#[proto_message(type_url = "/regen.ecocredit.basket.v1.DateCriteria")]
pub struct DateCriteria {
    #[prost(message, optional, tag = "1")]
    pub min_start_date: ::core::option::Option<crate::shim::Timestamp>,
    #[prost(message, optional, tag = "2")]
    pub start_date_window: ::core::option::Option<crate::shim::Duration>,
    #[prost(uint32, optional, tag = "3")]
    pub years_in_the_past: ::core::option::Option<u32>,
}

#[derive(
    Clone,
    PartialEq,
    Eq,
    ::prost::Message,
    ::serde::Serialize,
    ::serde::Deserialize,
    ::schemars::JsonSchema,
    CosmwasmExt,
)]
#[proto_message(type_url = "/regen.ecocredit.basket.v1.MsgCreate")]
pub struct MsgCreate {
    #[prost(string, tag = "1")]
    pub curator: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub name: ::prost::alloc::string::String,
    #[prost(string, tag = "3")]
    pub description: ::prost::alloc::string::String,
    #[prost(uint32, tag = "4")]
    pub exponent: u32,
    #[prost(bool, tag = "5")]
    pub disable_auto_retire: bool,
    #[prost(string, tag = "6")]
    pub credit_type_abbrev: ::prost::alloc::string::String,
    #[prost(string, repeated, tag = "7")]
    pub allowed_classes: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
    #[prost(message, optional, tag = "8")]
    pub date_criteria: ::core::option::Option<DateCriteria>,
    #[prost(message, repeated, tag = "9")]
    pub fee: ::prost::alloc::vec::Vec<Coin>,
}

#[test]
fn test_regen_protobuf_filter() {
    let mut suite = SuiteBuilder::base().build();
    let dao = suite.core_addr.clone();

    // Register the protobuf file descriptor set.

    let crate_root = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let proto_path = std::path::Path::new(&crate_root).join("proto/regen_ecocredit.pb");
    let file_descriptor_set = std::fs::read(proto_path).unwrap();

    // Create a role with an authorization that allows a true BoolValue.
    let role_id = suite.create_role(
        &dao,
        "basket creator",
        None,
        None,
        None,
        Some(vec![ADDR0.to_string()]),
    );

    let filter = Some(serde_json::json!({
        "stargate": {
            "type_url": "/regen.ecocredit.basket.v1.MsgCreate",
            "value": {
                "#proto": {
                    "type": "regen.ecocredit.basket.v1.MsgCreate",
                    "value": {
                        "curator": dao.to_string(),
                        "disableAutoRetire": {
                            "$or": [
                                { "$exists": false },
                                false,
                            ],
                        },
                        "creditTypeAbbrev": "C",
                        "allowedClasses": ["C-1", "C-2"],
                        "dateCriteria": {
                            // In July 2025
                            "minStartDate": {
                                "$startsWith": "2025-07-"
                            },
                            "startDateWindow": {
                                "#replace": {
                                    // Remove seconds suffix
                                    "find": "s",
                                    "replace": "",
                                    "filter": {
                                        "#to_number": {
                                            // Between 0 and 30 days
                                            "$between": [0, 2592000]
                                        }
                                    }
                                }
                            },
                            "yearsInThePast": {
                                "$or": [
                                    0,
                                    { "$exists": false },
                                ]
                            },
                        },
                        // The current basket creation fee.
                        "fee": [{
                            "denom": "uregen",
                            "amount": "1000000000"
                        }]
                    }
                }
            }
        }
    }));

    // Fail to create authorization if protobuf message is not registered.
    let err = suite.create_authorization_err(
        &dao,
        role_id,
        "create basket",
        None,
        filter.clone(),
        Some(true),
    );
    assert_eq!(
        err,
        ContractError::Std(StdError::generic_err(format!(
            "Querier contract error: {}",
            StdError::generic_err(
                cw_protobuf_registry::ContractError::ProtobufMessageNotFound {
                    message: "regen.ecocredit.basket.v1.MsgCreate".to_string(),
                }
                .to_string()
            )
        )))
    );

    // Register the protobuf file descriptor set.
    suite.register_protobufs(&dao, vec![file_descriptor_set.clone()]);

    // Successfully create an authorization that allows creating a basket with
    // specific parameter restrictions.
    let authorization_id =
        suite.create_authorization(&dao, role_id, "create basket", None, filter, Some(true));

    // Ensure the authorization has the correct protobuf message.
    let authorization = suite.get_authorization(authorization_id);
    assert_eq!(
        authorization.authorization.protobuf_messages,
        vec!["regen.ecocredit.basket.v1.MsgCreate".to_string()]
    );

    let pool = DescriptorPool::from_file_descriptor_set(
        FileDescriptorSet::decode(file_descriptor_set.as_slice()).unwrap(),
    )
    .unwrap();

    let base64_encoded_pass = base64_encode_protobuf(
        &pool,
        "regen.ecocredit.basket.v1.MsgCreate",
        &serde_json::json!({
            "curator": dao.to_string(),
            "disableAutoRetire": false,
            "creditTypeAbbrev": "C",
            "allowedClasses": ["C-1", "C-2"],
            "dateCriteria": {
                "minStartDate": "2025-07-02T00:00:00.000Z",
                "startDateWindow": "1s",
                "yearsInThePast": 0,
            },
            "fee": [{
                "denom": "uregen",
                "amount": "1000000000"
            }]
        }),
    );
    suite.assert_msg_authorized_by(
        ADDR0,
        role_id,
        authorization_id,
        &CosmosMsg::Stargate {
            type_url: "/regen.ecocredit.basket.v1.MsgCreate".to_string(),
            value: Binary::from_base64(&base64_encoded_pass).unwrap(),
        },
    );

    let base64_encoded_not_pass = base64_encode_protobuf(
        &pool,
        "regen.ecocredit.basket.v1.MsgCreate",
        &serde_json::json!({
            "curator": dao.to_string(),
            "disableAutoRetire": false,
            "creditTypeAbbrev": "C",
            "allowedClasses": ["C-1", "C-2"],
            "dateCriteria": {
                "minStartDate": "2025-08-02T00:00:00.000Z",
                "startDateWindow": "1s",
                "yearsInThePast": 0,
            },
            "fee": [{
                "denom": "uregen",
                "amount": "1000000000"
            }]
        }),
    );
    suite.assert_msg_unauthorized_by(
        ADDR0,
        role_id,
        authorization_id,
        &CosmosMsg::Stargate {
            type_url: "/regen.ecocredit.basket.v1.MsgCreate".to_string(),
            value: Binary::from_base64(&base64_encoded_not_pass).unwrap(),
        },
        Some(ContractError::MsgNotAllowedByFilter {
            err: cw_jsonfilter::FilterResult::operator_failed(
                "$startsWith",
                "value does not start with filter value",
                "@.stargate.value.#proto.dateCriteria.minStartDate.$startsWith",
                "@.stargate.value.#proto.dateCriteria.minStartDate",
            )
            .as_fail()
            .unwrap()
            .to_string(),
        }),
    );
}
