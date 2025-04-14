use std::convert::TryFrom;
use std::convert::TryInto;

use osmosis_std_derive::CosmwasmExt;

use crate::cosmos::{Coin, Metadata};

// see https://gitlab.com/thorchain/rujira/-/blob/main/packages/rujira-rs/src/msg/token_factory.rs

/// MsgCreateDenom is the sdk.Msg type for allowing an account to create
/// a new denom.  It requires a sender address and a unique nonce
/// (to allow accounts to create multiple denoms)
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
#[proto_message(type_url = "/thorchain.denom.v1.MsgCreateDenom")]
pub struct MsgCreateDenom {
    #[prost(string, tag = "1")]
    pub sender: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub id: ::prost::alloc::string::String,
    #[prost(message, required, tag = "3")]
    pub metadata: Metadata,
}

/// MsgCreateDenomResponse is the return value of MsgCreateDenom
/// It returns the full string of the newly created denom
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
#[proto_message(type_url = "/thorchain.denom.v1.MsgCreateDenomResponse")]
pub struct MsgCreateDenomResponse {
    #[prost(string, tag = "1")]
    pub new_token_denom: ::prost::alloc::string::String,
}

/// MsgMintTokens is the sdk.Msg type for allowing an admin account to mint
/// more of a token.
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
#[proto_message(type_url = "/thorchain.denom.v1.MsgMintTokens")]
pub struct MsgMintTokens {
    #[prost(string, tag = "1")]
    pub sender: ::prost::alloc::string::String,
    #[prost(message, required, tag = "2")]
    pub amount: Coin,
    #[prost(string, tag = "3")]
    pub recipient: ::prost::alloc::string::String,
}

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
#[proto_message(type_url = "/thorchain.denom.v1.MsgMintTokensResponse")]
pub struct MsgMintTokensResponse {}

/// MsgBurnTokens is the sdk.Msg type for allowing an admin account to burn
/// a token.  For now, we only support burning from the sender account.
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
#[proto_message(type_url = "/thorchain.denom.v1.MsgBurnTokens")]
pub struct MsgBurnTokens {
    #[prost(string, tag = "1")]
    pub sender: ::prost::alloc::string::String,
    #[prost(message, required, tag = "2")]
    pub amount: Coin,
}

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
#[proto_message(type_url = "/thorchain.denom.v1.MsgBurnTokensResponse")]
pub struct MsgBurnTokensResponse {}

/// MsgChangeAdmin is the sdk.Msg type for allowing an admin account to reassign
/// adminship of a denom to a new account
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
#[proto_message(type_url = "/thorchain.denom.v1.MsgChangeDenomAdmin")]
pub struct MsgChangeDenomAdmin {
    #[prost(string, tag = "1")]
    pub sender: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub denom: ::prost::alloc::string::String,
    #[prost(string, tag = "3")]
    pub new_admin: ::prost::alloc::string::String,
}

// MsgChangeAdminResponse defines the response structure for an executed
// MsgChangeAdmin message.
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
#[proto_message(type_url = "/thorchain.denom.v1.MsgChangeDenomAdminResponse")]
pub struct MsgChangeDenomAdminResponse {}
