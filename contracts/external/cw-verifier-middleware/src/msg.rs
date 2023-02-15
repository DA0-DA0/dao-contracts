use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Binary, Uint128};
use cw_utils::Expiration;

#[cw_serde]
pub struct WrappedMessage {
    pub payload: Payload,
    // Assumes 'payload' has been hashed, signed, and base64 encoded
    pub signature: Binary,
    pub public_key: String,
}

#[cw_serde]
pub struct Payload {
    pub nonce: Uint128,
    pub contract_address: String,
    pub msg: Binary,
    pub expiration: Option<Expiration>,
}
