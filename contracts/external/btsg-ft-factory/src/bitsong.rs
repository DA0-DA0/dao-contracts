use osmosis_std_derive::CosmwasmExt;

/// Coin defines a token with a denomination and an amount.
///
/// NOTE: The amount field is an Int which implements the custom method
/// signatures required by gogoproto.
#[derive(
    Clone,
    PartialEq,
    Eq,
    ::prost::Message,
    ::serde::Serialize,
    ::serde::Deserialize,
    schemars::JsonSchema,
    CosmwasmExt,
)]
#[proto_message(type_url = "/cosmos.base.v1beta1.Coin")]
pub struct Coin {
    #[prost(string, tag = "1")]
    pub denom: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub amount: ::prost::alloc::string::String,
}

// see https://github.com/bitsongofficial/go-bitsong/blob/dfa3563dccf990eac1d9dc4462c2850b9b2a21e1/proto/bitsong/fantoken/v1beta1/tx.proto

/// MsgIssue defines a message for issuing a new fan token
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
#[proto_message(type_url = "/bitsong.fantoken.v1beta1.MsgIssue")]
pub struct MsgIssue {
    #[prost(string, tag = "1")]
    pub symbol: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub name: ::prost::alloc::string::String,
    #[prost(string, tag = "3")]
    pub max_supply: ::prost::alloc::string::String,
    #[prost(string, tag = "4")]
    pub authority: ::prost::alloc::string::String,
    #[prost(string, tag = "5")]
    pub minter: ::prost::alloc::string::String,
    #[prost(string, tag = "6")]
    pub uri: ::prost::alloc::string::String,
}

/// MsgIssueResponse defines the MsgIssue response type
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
#[proto_message(type_url = "/bitsong.fantoken.v1beta1.MsgIssueResponse")]
pub struct MsgIssueResponse {}
