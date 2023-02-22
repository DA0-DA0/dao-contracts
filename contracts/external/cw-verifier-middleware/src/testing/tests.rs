use cosmwasm_std::{
    testing::{mock_dependencies, mock_env, mock_info},
    to_binary, Addr, HexBinary, Uint128,
};
use secp256k1::{rand::rngs::OsRng, Message, Secp256k1};
use sha2::{Digest, Sha256};

use crate::{
    error::ContractError,
    msg::{Payload, WrappedMessage},
    state::NONCES,
    verify::{pk_to_addr, verify},
};

#[test]
fn test_pk_to_addr_uncompressed() {
    let juno_address = Addr::unchecked("juno1muw4rz9ml44wc6vssqrzkys4nuc3gylrxj4flw");
    let juno_pk = "04f620cd2e33d3f6af5a43d5b3ca3b9b7f653aa980ae56714cc5eb7637fd1eeb28fb722c0dacb5f005f583630dae8bbe7f5eaba70f129fc279d7ff421ae8c9eb79".to_string();

    let deps = mock_dependencies();
    let generated_address = pk_to_addr(deps.as_ref(), juno_pk, &"juno").unwrap();
    assert_eq!(generated_address, juno_address);
}

#[test]
fn test_pk_to_addr_compressed() {
    let juno_address = Addr::unchecked("juno1vqxvyw6kpy7xj0msxz57svwn4p0kfdu46kv0pk");
    let juno_pk = "022bf538609c68cd00c931353602d9e3585732c078fe784ad2e4f29cf486a6afa2".to_string();

    let deps = mock_dependencies();
    let generated_address = pk_to_addr(deps.as_ref(), juno_pk, &"juno").unwrap();
    assert_eq!(generated_address, juno_address);
}

#[test]
fn test_verify_success() {
    // This test generates a payload in which the signature is of base64 format, and the public key is of hex format.
    // The test then calls verify to validate that the signature is correctly verified.

    let payload = Payload {
        nonce: Uint128::from(0u128),
        msg: to_binary("eyJpbnN0YW50aWF0ZV9jb250cmFjdF93aXRoX3NlbGZfYWRtaW4iOnsiY29kZV9pZCI6MTY4OCwiaW5zdGFudGlhdGVfbXNnIjp7fX19ICA=").unwrap(),
        expiration: None,
        contract_address: Addr::unchecked("contract_address").to_string(),
        bech32_prefix: "juno".to_string(),
        version: "version-1".to_string(),
    };

    // Generate a keypair
    let secp = Secp256k1::new();
    let (secret_key, public_key) = secp.generate_keypair(&mut OsRng);

    // Hash and sign the payload
    let msg_hash = Sha256::digest(&to_binary(&payload).unwrap());
    let msg = Message::from_slice(&msg_hash).unwrap();
    let sig = secp.sign_ecdsa(&msg, &secret_key);

    // Wrap the message
    let hex_encoded = HexBinary::from(public_key.serialize_uncompressed());
    let wrapped_msg = WrappedMessage {
        payload,
        signature: sig.serialize_compact().into(),
        public_key: hex_encoded.clone(),
    };

    // Verify
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info("creator", &[]);
    verify(deps.as_mut(), env, info, wrapped_msg).unwrap();

    // Verify nonce was incremented correctly
    let nonce = NONCES.load(&deps.storage, &hex_encoded.to_hex()).unwrap();
    assert_eq!(nonce, Uint128::from(1u128))
}

#[test]
fn test_verify_pk_invalid() {
    // This test generates a payload in which the signature is of base64 format, and the public key is of hex format.
    // The test then calls verify with an incorrectly formatted public key to validate that there is an error in parsing the public key.

    let payload = Payload {
        nonce: Uint128::from(0u128),
        msg: to_binary("test").unwrap(),
        expiration: None,
        contract_address: Addr::unchecked("contract_address").to_string(),
        bech32_prefix: "juno".to_string(),
        version: "version-1".to_string(),
    };

    // Generate a keypair
    let secp = Secp256k1::new();
    let (secret_key, _) = secp.generate_keypair(&mut OsRng);

    // Hash and sign the payload
    let msg_hash = Sha256::digest(&to_binary(&payload).unwrap());
    let msg = Message::from_slice(&msg_hash).unwrap();
    let sig = secp.sign_ecdsa(&msg, &secret_key);

    // Wrap the message but with incorrect public key
    let wrapped_msg = WrappedMessage {
        payload,
        signature: sig.serialize_compact().into(),
        public_key: Vec::from("incorrect_public_key").into(),
    };

    // Verify with incorrect public key
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info("creator", &[]);
    let result = verify(deps.as_mut(), env, info, wrapped_msg);

    // Ensure that there was a pub key verification error
    assert!(matches!(result, Err(ContractError::VerificationError(_))));
}

#[test]
fn test_verify_wrong_pk() {
    // This test generates a payload in which the signature is of base64 format, and the public key is of hex format.
    // The test then calls verify with an correctly formatted but different public key (that doesn't correspond to the signature) to validate that the signature is not verified.

    let payload = Payload {
        nonce: Uint128::from(0u128),
        msg: to_binary("test").unwrap(),
        expiration: None,
        contract_address: Addr::unchecked("contract_address").to_string(),
        bech32_prefix: "juno".to_string(),
        version: "version-1".to_string(),
    };

    // Generate a keypair
    let secp = Secp256k1::new();
    let (secret_key, _) = secp.generate_keypair(&mut OsRng);

    // Hash and sign the payload
    let msg_hash = Sha256::digest(&to_binary(&payload).unwrap());
    let msg = Message::from_slice(&msg_hash).unwrap();
    let sig = secp.sign_ecdsa(&msg, &secret_key);

    // Generate another keypair
    let secp = Secp256k1::new();
    let (_, public_key) = secp.generate_keypair(&mut OsRng);

    // Wrap the message but with incorrect public key
    let wrapped_msg = WrappedMessage {
        payload,
        signature: sig.serialize_compact().into(),
        public_key: HexBinary::from(public_key.serialize_uncompressed()),
    };

    // Verify with incorrect public key
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info("creator", &[]);
    let result = verify(deps.as_mut(), env, info, wrapped_msg);

    // Ensure that there was a signature verification error
    assert!(matches!(result, Err(ContractError::SignatureInvalid)));
}

/*
Moar tests to write:
signature is invalid / malformed
incorrect nonce
expired message
load a keypair corresponding to pre-known address and validate that address in info was set correctly
*/
