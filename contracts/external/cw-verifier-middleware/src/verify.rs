use crate::{
    error::ContractError,
    msg::WrappedMessage,
    state::{CONTRACT_ADDRESS, NONCES},
};
use bech32::{ToBase32, Variant};
use cosmwasm_schema::{cw_serde, serde::Serialize};
use cosmwasm_std::{
    to_binary, Addr, Binary, DepsMut, Env, MessageInfo, OverflowError, StdError, Timestamp, Uint128,
};
use cw_utils::Expiration;
use ripemd::Ripemd160;
use secp256k1::{ecdsa::Signature, Message as SecpMessage, PublicKey, Secp256k1};
use sha2::{Digest, Sha256};

pub const ADDR_PREFIX: &str = "juno";

pub fn verify(
    deps: DepsMut,
    env: Env,
    mut info: MessageInfo,
    wrapped_msg: WrappedMessage,
) -> Result<Binary, ContractError> {
    // Serialize the inner message
    let msg_ser = to_binary(&wrapped_msg.payload)?;

    // Hash the serialized payload using SHA-256
    let msg_hash = Sha256::digest(&msg_ser);

    println!("hex public key: {:?}", wrapped_msg.public_key.as_bytes());
    // Verify the signature
    let sig_valid = deps.api.secp256k1_verify(
        msg_hash.as_slice(),
        &wrapped_msg.signature,
        to_binary(&wrapped_msg.public_key)?.as_slice(),
    )?;

    println!("after verification");

    if !sig_valid {
        return Err(ContractError::SignatureInvalid {});
    }

    // Validate that the message has the correct nonce
    let nonce = NONCES
        .may_load(deps.storage, &wrapped_msg.public_key)?
        .unwrap_or(Uint128::from(0u128));

    if wrapped_msg.payload.nonce != nonce {
        return Err(ContractError::InvalidNonce {});
    }

    // Increment nonce
    NONCES.update(
        deps.storage,
        &wrapped_msg.public_key,
        |nonce: Option<Uint128>| {
            nonce
                .unwrap_or(Uint128::from(0u128))
                .checked_add(Uint128::from(1u128))
                .map_err(|e| StdError::from(e))
        },
    )?;

    // Validate that the message has not expired
    if let Some(expiration) = wrapped_msg.payload.expiration {
        if expiration.is_expired(&env.block) {
            return Err(ContractError::MessageExpired {});
        }
    }

    println!("{:?}", wrapped_msg.public_key);

    // Set the message sender to the address corresponding to the provided public key. (pk_to_addr)
    info.sender = ec_pk_to_bech32_address(wrapped_msg.public_key.to_string(), ADDR_PREFIX)?;

    // Return the msg; caller will deserialize
    return Ok(wrapped_msg.payload.msg);
}

pub fn initialize_contract_addr(deps: DepsMut, env: Env) -> Result<(), ContractError> {
    CONTRACT_ADDRESS.save(deps.storage, &env.contract.address.to_string())?;
    Ok(())
}

// takes an uncompressed EC public key and a prefix
pub fn ec_pk_to_bech32_address(hex_pk: String, prefix: &str) -> Result<Addr, ContractError> {
    println!("hex_pk: {:?}", hex_pk.len());
    if hex_pk.clone().len() != 130 {
        return Err(ContractError::Std(StdError::InvalidHex {
            msg: "unexpected hex encoded uncompressed public key length".to_string(),
        }));
    }

    // get the raw public key bytes
    let decoded_pk = hex::decode(hex_pk);
    let raw_pk = match decoded_pk {
        Ok(pk) => pk,
        Err(e) => {
            return Err(ContractError::Std(StdError::InvalidHex {
                msg: e.to_string(),
            }))
        }
    };

    // extract the compressed version of public key
    let public_key = secp256k1::PublicKey::from_slice(raw_pk.as_slice());
    let raw_pk = match public_key {
        Ok(pk) => pk.serialize().to_vec(),
        Err(e) => {
            return Err(ContractError::Std(StdError::GenericErr {
                msg: e.to_string(),
            }))
        }
    };

    // sha256 the raw public key
    let pk_sha256 = Sha256::digest(raw_pk);

    // take the ripemd160 of the sha256 of the raw pk
    let address_raw = Ripemd160::digest(pk_sha256);

    // encode the prefix and the raw address bytes with Bech32
    let bech32 = bech32::encode(&prefix, address_raw.to_base32(), Variant::Bech32);

    match bech32 {
        Ok(addr) => Ok(Addr::unchecked(addr)),
        Err(e) => Err(ContractError::Std(StdError::generic_err(e.to_string()))),
    }
}
