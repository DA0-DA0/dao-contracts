use cosmwasm_std::{to_json_binary, CosmosMsg, Deps, QueryRequest, StdError, StdResult};
use osmosis_std_derive::CosmwasmExt;
use std::convert::{TryFrom, TryInto};

use ::serde::{Deserialize, Deserializer, Serialize, Serializer};
use chrono::{DateTime, NaiveDateTime, Utc};
use serde::de;
use serde::de::Visitor;

use std::fmt;
use std::str::FromStr;

// see https://github.com/OmniFlix/omniflixhub/blob/main/proto/OmniFlix/onft/v1beta1

/// ONFT info
#[derive(
    Clone,
    PartialEq,
    Eq,
    ::prost::Message,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
)]
pub struct Onft {
    #[prost(string, tag = "1")]
    pub id: ::prost::alloc::string::String,
    #[prost(message, tag = "2")]
    pub metadata: ::core::option::Option<Metadata>,
    #[prost(string, tag = "3")]
    pub data: ::prost::alloc::string::String,
    #[prost(string, tag = "4")]
    pub owner: ::prost::alloc::string::String,
    #[prost(bool, tag = "5")]
    pub transferable: bool,
    #[prost(bool, tag = "6")]
    pub extensible: bool,
    #[prost(message, tag = "7")]
    pub created_at: ::core::option::Option<Timestamp>,
    #[prost(bool, tag = "8")]
    pub nsfw: bool,
    #[prost(string, tag = "9")]
    pub royalty_share: ::prost::alloc::string::String,
}

/// ONFT metadata
#[derive(
    Clone,
    PartialEq,
    Eq,
    ::prost::Message,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
)]
pub struct Metadata {
    #[prost(string, tag = "1")]
    pub name: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub description: ::prost::alloc::string::String,
    #[prost(string, tag = "3")]
    pub media_uri: ::prost::alloc::string::String,
    #[prost(string, tag = "4")]
    pub preview_uri: ::prost::alloc::string::String,
    #[prost(string, tag = "5")]
    pub uri_hash: ::prost::alloc::string::String,
}

/// QueryONFTRequest requests the info for a single ONFT.
#[derive(
    Clone,
    PartialEq,
    Eq,
    ::prost::Message,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
    CosmwasmExt,
)]
#[proto_message(type_url = "/omniflix.onft.v1beta1.QueryONFTRequest")]
pub struct QueryONFTRequest {
    #[prost(string, tag = "1")]
    pub denom_id: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub id: ::prost::alloc::string::String,
}

/// QueryONFTResponse returns the info for a single ONFT.
#[derive(
    Clone,
    PartialEq,
    Eq,
    ::prost::Message,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
    CosmwasmExt,
)]
#[proto_message(type_url = "/omniflix.onft.v1beta1.QueryONFTResponse")]
pub struct QueryONFTResponse {
    #[prost(message, tag = "1")]
    pub onft: ::core::option::Option<Onft>,
}

/// QuerySupplyRequest requests the supply of the denom.
#[derive(
    Clone,
    PartialEq,
    Eq,
    ::prost::Message,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
    CosmwasmExt,
)]
#[proto_message(type_url = "/omniflix.onft.v1beta1.QuerySupplyRequest")]
pub struct QuerySupplyRequest {
    #[prost(string, tag = "1")]
    pub denom_id: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub owner: ::prost::alloc::string::String,
}

/// QuerySupplyResponse returns the supply of the denom.
#[derive(
    Clone,
    PartialEq,
    Eq,
    ::prost::Message,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
    CosmwasmExt,
)]
#[proto_message(type_url = "/omniflix.onft.v1beta1.QuerySupplyResponse")]
pub struct QuerySupplyResponse {
    #[prost(uint64, tag = "1")]
    pub amount: u64,
}

/// MsgTransferONFT transfers an ONFT.
#[derive(
    Clone,
    PartialEq,
    Eq,
    ::prost::Message,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
    CosmwasmExt,
)]
#[proto_message(type_url = "/omniflix.onft.v1beta1.MsgTransferONFT")]
pub struct MsgTransferONFT {
    #[prost(string, tag = "1")]
    pub id: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub denom_id: ::prost::alloc::string::String,
    #[prost(string, tag = "3")]
    pub sender: ::prost::alloc::string::String,
    #[prost(string, tag = "4")]
    pub recipient: ::prost::alloc::string::String,
}

/// MsgTransferONFTResponse is the return type of MsgTransferONFT.
#[derive(
    Clone,
    PartialEq,
    Eq,
    ::prost::Message,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
    CosmwasmExt,
)]
#[proto_message(type_url = "/omniflix.onft.v1beta1.MsgTransferONFTResponse")]
pub struct MsgTransferONFTResponse {}

#[derive(Clone, PartialEq, Eq, ::prost::Message, schemars::JsonSchema)]
pub struct Timestamp {
    /// Represents seconds of UTC time since Unix epoch
    /// 1970-01-01T00:00:00Z. Must be from 0001-01-01T00:00:00Z to
    /// 9999-12-31T23:59:59Z inclusive.
    #[prost(int64, tag = "1")]
    pub seconds: i64,
    /// Non-negative fractions of a second at nanosecond resolution. Negative
    /// second values with fractions must still have non-negative nanos values
    /// that count forward in time. Must be from 0 to 999,999,999
    /// inclusive.
    #[prost(int32, tag = "2")]
    pub nanos: i32,
}

impl Serialize for Timestamp {
    fn serialize<S>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
    where
        S: Serializer,
    {
        let mut ts = prost_types::Timestamp {
            seconds: self.seconds,
            nanos: self.nanos,
        };
        ts.normalize();
        let dt = NaiveDateTime::from_timestamp_opt(ts.seconds, ts.nanos as u32)
            .expect("invalid or out-of-range datetime");
        let dt: DateTime<Utc> = DateTime::from_naive_utc_and_offset(dt, Utc);
        serializer.serialize_str(format!("{:?}", dt).as_str())
    }
}

impl<'de> Deserialize<'de> for Timestamp {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        struct TimestampVisitor;

        impl<'de> Visitor<'de> for TimestampVisitor {
            type Value = Timestamp;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("Timestamp in RFC3339 format")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                let utc: DateTime<Utc> = chrono::DateTime::from_str(value).map_err(|err| {
                    serde::de::Error::custom(format!(
                        "Failed to parse {} as datetime: {:?}",
                        value, err
                    ))
                })?;
                let ts = Timestamp::from(utc);
                Ok(ts)
            }
        }
        deserializer.deserialize_str(TimestampVisitor)
    }
}

impl From<DateTime<Utc>> for Timestamp {
    fn from(dt: DateTime<Utc>) -> Self {
        Timestamp {
            seconds: dt.timestamp(),
            nanos: dt.timestamp_subsec_nanos() as i32,
        }
    }
}

pub fn query_onft_owner(deps: Deps, denom_id: &str, token_id: &str) -> StdResult<String> {
    let res: QueryONFTResponse = deps.querier.query(&QueryRequest::Stargate {
        path: "/omniflix.onft.v1beta1.Query/ONFT".to_string(),
        data: to_json_binary(&QueryONFTRequest {
            denom_id: denom_id.to_string(),
            id: token_id.to_string(),
        })?,
    })?;

    let owner = res
        .onft
        .ok_or(StdError::generic_err("ONFT not found"))?
        .owner;

    Ok(owner)
}

pub fn query_onft_supply(deps: Deps, id: &str) -> StdResult<u64> {
    let res: QuerySupplyResponse = deps.querier.query(&QueryRequest::Stargate {
        path: "/omniflix.onft.v1beta1.Query/Supply".to_string(),
        data: to_json_binary(&QuerySupplyRequest {
            denom_id: id.to_string(),
            owner: "".to_string(),
        })?,
    })?;

    Ok(res.amount)
}

pub fn get_onft_transfer_msg(
    denom_id: &str,
    token_id: &str,
    sender: &str,
    recipient: &str,
) -> CosmosMsg {
    MsgTransferONFT {
        denom_id: denom_id.to_string(),
        id: token_id.to_string(),
        sender: sender.to_string(),
        recipient: recipient.to_string(),
    }
    .into()
}
