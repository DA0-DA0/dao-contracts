use crate::{
    error::ContractError,
    msg::WrappedMessage,
    state::{CONTRACT_ADDRESS, NONCES},
};
use bech32::{ToBase32, Variant};
use cosmwasm_schema::{schemars::_serde_json::json, serde::de::DeserializeOwned};
use cosmwasm_std::{
    from_slice, to_binary, Addr, Api, DepsMut, Env, MessageInfo, StdError, Storage, Uint128,
};

use ripemd::Ripemd160;
use sha2::{Digest, Sha256};

const UNCOMPRESSED_HEX_PK_LEN: usize = 130;
const COMPRESSED_HEX_PK_LEN: usize = 66;

pub fn verify<T>(
    api: &dyn Api,
    storage: &mut dyn Storage,
    env: &Env,
    info: MessageInfo,
    wrapped_msg: WrappedMessage,
) -> Result<(T, MessageInfo), ContractError>
where
    T: DeserializeOwned,
{
    let payload = wrapped_msg.payload;

    let signer_addr = pk_to_addr(
        api,
        wrapped_msg.public_key.to_hex(), // to_hex ensures that the public key has the expected number of bytes
        &payload.bech32_prefix,
    )?;

    let payload_ser = serde_json::to_string(&payload)?;

    // Convert message to signDoc format
    let sign_doc = get_sign_doc(&signer_addr.as_str(), &payload_ser, &payload.chain_id)?;

    // Serialize the payload
    let msg_ser = to_binary(&sign_doc)?;

    // Hash the serialized payload using SHA-256
    let msg_hash = Sha256::digest(&msg_ser);

    // Verify the signature
    let sig_valid = api.secp256k1_verify(
        msg_hash.as_slice(),
        &wrapped_msg.signature,
        wrapped_msg.public_key.as_slice(),
    )?;

    if !sig_valid {
        return Err(ContractError::SignatureInvalid {});
    }

    // Validate that the message has not expired
    if let Some(expiration) = payload.expiration {
        if expiration.is_expired(&env.block) {
            return Err(ContractError::MessageExpired {});
        }
    }

    let validated_contract_addr = api.addr_validate(&payload.contract_address)?;
    let pk = wrapped_msg.public_key.to_hex();
    let nonce_key = (
        pk.as_str(),
        &validated_contract_addr,
        payload.contract_version.as_str(),
    );

    // Validate that the message has the correct nonce
    let nonce = NONCES
        .may_load(storage, nonce_key)?
        .unwrap_or(Uint128::from(0u128));

    if payload.nonce != nonce {
        return Err(ContractError::InvalidNonce {});
    }

    // Increment nonce
    NONCES.update(storage, nonce_key, |nonce: Option<Uint128>| {
        nonce
            .unwrap_or(Uint128::from(0u128))
            .checked_add(Uint128::from(1u128))
            .map_err(|e| StdError::from(e))
    })?;

    // Construct a new MessageInfo with the signer as the sender
    let verified_info = MessageInfo {
        sender: signer_addr,
        funds: info.funds,
    };

    // Deserialize message into expected type
    let verified_msg = from_slice::<T>(&payload.msg.to_vec())?;

    // Return info with sender and deserialized msg
    return Ok((verified_msg, verified_info));
}

pub fn initialize_contract_addr(deps: DepsMut, env: &Env) -> Result<(), ContractError> {
    CONTRACT_ADDRESS.save(deps.storage, &env.contract.address.to_string())?;
    Ok(())
}

// Takes an compressed or uncompressed hex-encoded EC public key and a bech32 prefix and derives the bech32 address.
pub fn pk_to_addr(api: &dyn Api, hex_pk: String, prefix: &str) -> Result<Addr, ContractError> {
    // Decode PK from hex
    let raw_pk = hex::decode(&hex_pk)?;

    let raw_pk: Vec<u8> = match hex_pk.len() {
        COMPRESSED_HEX_PK_LEN => Ok::<std::vec::Vec<u8>, ContractError>(raw_pk),
        UNCOMPRESSED_HEX_PK_LEN => {
            let public_key = secp256k1::PublicKey::from_slice(raw_pk.as_slice())?;
            // serialize will convert pk to compressed format
            Ok(public_key.serialize().to_vec())
        }
        _ => {
            return Err(ContractError::InvalidPublicKeyLength {
                length: hex_pk.len(),
            })
        }
    }?;

    // sha256 hash the raw public key
    let pk_sha256 = Sha256::digest(raw_pk);

    // Take the ripemd160 of the sha256 of the raw pk
    let address_raw = Ripemd160::digest(pk_sha256);

    // Encode the prefix and the raw address bytes with bech32
    let bech32 = bech32::encode(&prefix, address_raw.to_base32(), Variant::Bech32)?;

    // Return validated addr
    Ok(api.addr_validate(&bech32)?)
}

use serde_json;

pub fn get_sign_doc(signer: &str, message: &str, chain_id: &str) -> Result<String, ContractError> {
    let doc = json!({
        "account_number": "0",
        "chain_id": chain_id,
        "fee": {
            "amount": [],
            "gas": "0"
        },
        "memo": "",
        "msgs": [
            {
                "type": "cw-verifier",
                "value": {
                    "data": message,
                    "signer": signer
                }
            }
        ],
        "sequence": "0"
    });

    Ok(serde_json::to_string(&doc)?)
}

pub fn execute_submit_externally_signed<
    T: DeserializeOwned,
    E: Error + From<crate::error::ContractError>,
>(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: WrappedMessage,
    execute: fn(DepsMut, Env, MessageInfo, T) -> Result<Response, E>,
) -> Result<Response, E> {
    let (msg, info): (T, _) = verify(deps.api, deps.storage, env, info, msg)?;
    execute(deps, env, info, msg)
}
