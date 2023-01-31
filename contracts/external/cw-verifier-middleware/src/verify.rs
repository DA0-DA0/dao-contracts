use cosmwasm_std::{Response, Binary, Timestamp
};
use crate::{error::ContractError, msg::ExecuteMsg};


pub fn execute_wrap(msg: Message) -> Result<Response, ContractError>{
    // 1. verify signature
    // secp256k1::verify(&msg.signature, &msg.payload, &msg.public_key)?;

    return Ok(Response::default())
}

pub struct Message {
    pub payload: Payload,
    pub signature: Binary, 
    pub public_key: secp256k1::PublicKey,
}

pub struct Payload {
    pub nonce: u64,
    pub msg: ExecuteMsg,
    pub expiration: Option<Timestamp>,
}