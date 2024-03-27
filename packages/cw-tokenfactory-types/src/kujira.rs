use std::convert::TryFrom;
use std::convert::TryInto;

use osmosis_std_derive::CosmwasmExt;

use crate::cosmos::Coin;

// see https://github.com/Team-Kujira/core/blob/f1c18afd68578be9e9fd33de9adb9a32acc0df99/proto/kujira/denom/tx.proto

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
#[proto_message(type_url = "/kujira.denom.MsgCreateDenom")]
pub struct MsgCreateDenom {
    #[prost(string, tag = "1")]
    pub sender: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub nonce: ::prost::alloc::string::String,
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
#[proto_message(type_url = "/kujira.denom.MsgCreateDenomResponse")]
pub struct MsgCreateDenomResponse {
    #[prost(string, tag = "1")]
    pub new_token_denom: ::prost::alloc::string::String,
}

/// MsgMint is the sdk.Msg type for allowing an admin account to mint
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
#[proto_message(type_url = "/kujira.denom.MsgMint")]
pub struct MsgMint {
    #[prost(string, tag = "1")]
    pub sender: ::prost::alloc::string::String,
    #[prost(message, tag = "2")]
    pub amount: ::core::option::Option<Coin>,
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
#[proto_message(type_url = "/kujira.denom.MsgMintResponse")]
pub struct MsgMintResponse {}

/// MsgBurn is the sdk.Msg type for allowing an admin account to burn
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
#[proto_message(type_url = "/kujira.denom.MsgBurn")]
pub struct MsgBurn {
    #[prost(string, tag = "1")]
    pub sender: ::prost::alloc::string::String,
    #[prost(message, tag = "2")]
    pub amount: ::core::option::Option<Coin>,
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
#[proto_message(type_url = "/kujira.denom.MsgBurnResponse")]
pub struct MsgBurnResponse {}

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
#[proto_message(type_url = "/kujira.denom.MsgChangeAdmin")]
pub struct MsgChangeAdmin {
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
#[proto_message(type_url = "/kujira.denom.MsgChangeAdminResponse")]
pub struct MsgChangeAdminResponse {}
