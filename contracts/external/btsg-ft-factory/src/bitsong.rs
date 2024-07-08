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
#[proto_message(type_url = "/bitsong.fantoken.MsgIssue")]
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
#[proto_message(type_url = "/bitsong.fantoken.MsgIssueResponse")]
pub struct MsgIssueResponse {
    #[prost(string, tag = "1")]
    pub denom: ::prost::alloc::string::String,
}

/// MsgMint defines a message for minting a new fan token
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
#[proto_message(type_url = "/bitsong.fantoken.MsgMint")]
pub struct MsgMint {
    #[prost(string, tag = "1")]
    pub recipient: ::prost::alloc::string::String,
    #[prost(message, tag = "2")]
    pub coin: ::core::option::Option<Coin>,
    #[prost(string, tag = "3")]
    pub minter: ::prost::alloc::string::String,
}

/// MsgMintResponse defines the MsgMint response type
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
#[proto_message(type_url = "/bitsong.fantoken.MsgMintResponse")]
pub struct MsgMintResponse {}

/// MsgSetMinter defines a message for changing the fan token minter address
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
#[proto_message(type_url = "/bitsong.fantoken.MsgSetMinter")]
pub struct MsgSetMinter {
    #[prost(string, tag = "1")]
    pub denom: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub old_minter: ::prost::alloc::string::String,
    #[prost(string, tag = "3")]
    pub new_minter: ::prost::alloc::string::String,
}

/// MsgSetMinterResponse defines the MsgSetMinter response type
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
#[proto_message(type_url = "/bitsong.fantoken.MsgSetMinterResponse")]
pub struct MsgSetMinterResponse {}

// MsgSetAuthority defines a message for changing the fan token minter address
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
#[proto_message(type_url = "/bitsong.fantoken.MsgSetAuthority")]
pub struct MsgSetAuthority {
    #[prost(string, tag = "1")]
    pub denom: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub old_authority: ::prost::alloc::string::String,
    #[prost(string, tag = "3")]
    pub new_authority: ::prost::alloc::string::String,
}

// MsgSetAuthorityResponse defines the MsgSetAuthority response type
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
#[proto_message(type_url = "/bitsong.fantoken.MsgSetAuthorityResponse")]
pub struct MsgSetAuthorityResponse {}

/// MsgSetUri defines a message for updating the fan token URI
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
#[proto_message(type_url = "/bitsong.fantoken.MsgSetUri")]
pub struct MsgSetUri {
    #[prost(string, tag = "1")]
    pub authority: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub denom: ::prost::alloc::string::String,
    #[prost(string, tag = "3")]
    pub uri: ::prost::alloc::string::String,
}

/// MsgSetUriResponse defines the MsgSetUri response type
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
#[proto_message(type_url = "/bitsong.fantoken.MsgSetUriResponse")]
pub struct MsgSetUriResponse {}
