use cosmwasm_std::{
    testing::{mock_dependencies, mock_env, mock_info},
    to_binary, Addr, BlockInfo, DepsMut, HexBinary, Uint128, VerificationError,
};
use cw_utils::Expiration;
use secp256k1::{hashes::hex::ToHex, rand::rngs::OsRng, Message, PublicKey, Secp256k1, SecretKey};
use sha2::{Digest, Sha256};

use crate::{
    error::ContractError,
    msg::{Payload, WrappedMessage},
    state::NONCES,
    verify::{get_sign_doc, pk_to_addr, verify},
};

pub const JUNO_ADDRESS: &str = "juno1muw4rz9ml44wc6vssqrzkys4nuc3gylrxj4flw";
pub const COMPRESSED_PK: &str =
    "03f620cd2e33d3f6af5a43d5b3ca3b9b7f653aa980ae56714cc5eb7637fd1eeb28";
pub const UNCOMPRESSED_PK: &str = "04f620cd2e33d3f6af5a43d5b3ca3b9b7f653aa980ae56714cc5eb7637fd1eeb28fb722c0dacb5f005f583630dae8bbe7f5eaba70f129fc279d7ff421ae8c9eb79";
pub const JUNO_PREFIX: &str = "juno";

#[test]
fn test_pk_to_addr_uncompressed() {
    let deps = mock_dependencies();
    let generated_address =
        pk_to_addr(deps.as_ref(), UNCOMPRESSED_PK.to_string(), JUNO_PREFIX).unwrap();

    assert_eq!(generated_address, Addr::unchecked(JUNO_ADDRESS));
}

#[test]
fn test_pk_to_addr_compressed() {
    let deps = mock_dependencies();
    let generated_address =
        pk_to_addr(deps.as_ref(), COMPRESSED_PK.to_string(), JUNO_PREFIX).unwrap();
    assert_eq!(generated_address, Addr::unchecked(JUNO_ADDRESS));
}

#[test]
fn test_pk_to_addr_invalid_hex_length() {
    let invalid_length_pk = "".to_string();
    let deps = mock_dependencies();
    let err: ContractError = pk_to_addr(deps.as_ref(), invalid_length_pk, JUNO_PREFIX).unwrap_err();

    assert!(matches!(err, ContractError::InvalidPublicKeyLength { .. }));
}

#[test]
fn test_pk_to_addr_not_hex_pk() {
    let non_hex_pk =
        "03zzzzcd2e33d3f6af5a43d5b3ca3b9b7f653aa980ae56714cc5eb7637fd1eeb28".to_string();
    let deps = mock_dependencies();
    let err: ContractError = pk_to_addr(deps.as_ref(), non_hex_pk, JUNO_PREFIX).unwrap_err();

    assert!(matches!(err, ContractError::FromHexError { .. }));
}

#[test]
fn test_pk_to_addr_bech32_invalid_human_readable_part() {
    let deps = mock_dependencies();
    let err: ContractError =
        pk_to_addr(deps.as_ref(), UNCOMPRESSED_PK.to_string(), "jUnO").unwrap_err();

    assert!(matches!(err, ContractError::Bech32Error { .. }));
}

#[test]
fn test_verify_success() {
    fn test_verify_success() {
        // This test generates a payload in which the signature is base64 encoded, and the public key is hex encoded.
        // The test then calls verify to validate that the signature is correctly verified.

        let payload = Payload {
        nonce: Uint128::from(0u128),
        msg: to_binary("eyJpbnN0YW50aWF0ZV9jb250cmFjdF93aXRoX3NlbGZfYWRtaW4iOnsiY29kZV9pZCI6MTY4OCwiaW5zdGFudGlhdGVfbXNnIjp7fX19ICA=").unwrap(),
        expiration: None,
        contract_address: Addr::unchecked("contract_address").to_string(),
        bech32_prefix: "juno".to_string(),
        contract_version: "version-1".to_string(),
        chain_id: "juno-1".to_string(),
    };

        let mut deps = mock_dependencies();
        let wrapped_msg = get_wrapped_msg(deps.as_mut(), payload.clone());

        // Verify
        let env = mock_env();
        let mut info = mock_info("creator", &[]);
        verify(deps.as_mut(), env, &mut info, wrapped_msg.clone()).unwrap();

        // Verify nonce was incremented correctly
        let nonce = NONCES
            .load(
                &deps.storage,
                (
                    &wrapped_msg.public_key.to_hex(),
                    &Addr::unchecked(payload.contract_address),
                    &payload.contract_version,
                ),
            )
            .unwrap();
        assert_eq!(nonce, Uint128::from(1u128))
    }
}

#[test]
fn test_verify_invalid_pk() {
    // This test generates a payload in which the signature is of base64 format, and the public key is of hex format.
    // The test then calls verify with an incorrectly formatted public key to validate that there is an error in parsing the public key.

    let payload = Payload {
        nonce: Uint128::from(0u128),
        msg: to_binary("test").unwrap(),
        expiration: None,
        contract_address: Addr::unchecked("contract_address").to_string(),
        bech32_prefix: "juno".to_string(),
        contract_version: "version-1".to_string(),
        chain_id: "juno-1".to_string(),
    };

    // Generate wrapped message
    let mut deps = mock_dependencies();
    let mut wrapped_msg = get_wrapped_msg(deps.as_mut(), payload);

    // Set public key to invalid
    wrapped_msg.public_key = Vec::from("incorrect_public_key").into();

    // Verify with incorrect public key
    let env = mock_env();
    let mut info = mock_info("creator", &[]);
    let result = verify(deps.as_mut(), env, &mut info, wrapped_msg);

    // Ensure that there was a pub key parsing error
    assert!(matches!(
        result,
        Err(ContractError::InvalidPublicKeyLength { .. })
    ));
}

#[test]
fn test_verify_wrong_pk() {
    let payload = Payload {
        nonce: Uint128::from(0u128),
        msg: to_binary("test").unwrap(),
        expiration: None,
        contract_address: Addr::unchecked("contract_address").to_string(),
        bech32_prefix: "juno".to_string(),
        contract_version: "version-1".to_string(),
        chain_id: "juno-1".to_string(),
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
    let mut info = mock_info("creator", &[]);
    let result = verify(deps.as_mut(), env, &mut info, wrapped_msg);

    // Ensure that there was a signature verification error
    assert!(matches!(result, Err(ContractError::SignatureInvalid)));
}

#[test]
fn test_verify_incorrect_nonce() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let mut info = mock_info("creator", &[]);

    // get a default wrapped message and verify it
    let payload = Payload {
        nonce: Uint128::from(0u128),
        msg: to_binary("eyJpbnN0YW50aWF0ZV9jb250cmFjdF93aXRoX3NlbGZfYWRtaW4iOnsiY29kZV9pZCI6MTY4OCwiaW5zdGFudGlhdGVfbXNnIjp7fX19ICA=").unwrap(),
        expiration: None,
        contract_address: Addr::unchecked("contract_address").to_string(),
        bech32_prefix: JUNO_PREFIX.to_string(),
        contract_version: "version-1".to_string(),
        chain_id: "juno-1".to_string(),
    };
    let wrapped_msg = get_wrapped_msg(deps.as_mut(), payload);
    verify(deps.as_mut(), env.clone(), &mut info, wrapped_msg).unwrap();

    // skip a nonce iteration
    let invalid_nonce_payload = Payload {
        nonce: Uint128::from(3u128),
        msg: to_binary("eyJpbnN0YW50aWF0ZV9jb250cmFjdF93aXRoX3NlbGZfYWRtaW4iOnsiY29kZV9pZCI6MTY4OCwiaW5zdGFudGlhdGVfbXNnIjp7fX19ICA=").unwrap(),
        expiration: None,
        contract_address: Addr::unchecked("contract_address").to_string(),
        bech32_prefix: JUNO_PREFIX.to_string(),
        contract_version: "version-1".to_string(),
        chain_id: "juno-1".to_string(),
    };
    let wrapped_msg = get_wrapped_msg(deps.as_mut(), invalid_nonce_payload);
    let err = verify(deps.as_mut(), env, &mut info, wrapped_msg).unwrap_err();

    // verify the invalid nonce error
    assert!(matches!(err, ContractError::InvalidNonce));
}

#[test]
fn test_verify_expired_message() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let mut info = mock_info("creator", &[]);

    // get an expired message
    let payload = Payload {
        nonce: Uint128::from(0u128),
        msg: to_binary("eyJpbnN0YW50aWF0ZV9jb250cmFjdF93aXRoX3NlbGZfYWRtaW4iOnsiY29kZV9pZCI6MTY4OCwiaW5zdGFudGlhdGVfbXNnIjp7fX19ICA=").unwrap(),
        expiration: Some(cw_utils::Expiration::AtHeight(0)),
        contract_address: Addr::unchecked("contract_address").to_string(),
        bech32_prefix: JUNO_PREFIX.to_string(),
        contract_version: "version-1".to_string(),
        chain_id: "juno-1".to_string(),
    };
    let wrapped_msg = get_wrapped_msg(deps.as_mut(), payload);

    let err: ContractError =
        verify(deps.as_mut(), env.clone(), &mut info, wrapped_msg).unwrap_err();

    assert!(matches!(err, ContractError::MessageExpired));
}

#[test]
fn test_verify_wrong_payload() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let mut info = mock_info("creator", &[]);

    // Generate a keypair
    let secp = Secp256k1::new();
    let (secret_key, public_key) = secp.generate_keypair(&mut OsRng);

    let payload = Payload {
        nonce: Uint128::from(0u128),
        msg: to_binary("eyJpbnN0YW50aWF0ZV9jb250cmFjdF93aXRoX3NlbGZfYWRtaW4iOnsiY29kZV9pZCI6MTY4OCwiaW5zdGFudGlhdGVfbXNnIjp7fX19ICA=").unwrap(),
        expiration: None,
        contract_address: Addr::unchecked("contract_address").to_string(),
        bech32_prefix: JUNO_PREFIX.to_string(),
        contract_version: "version-1".to_string(),
        chain_id: "juno-1".to_string(),
    };

    // Hash and sign the payload
    let msg_hash = Sha256::digest(&to_binary(&payload).unwrap());
    let msg = Message::from_slice(&msg_hash).unwrap();
    let sig = secp.sign_ecdsa(&msg, &secret_key);

    let hex_encoded = HexBinary::from(public_key.serialize_uncompressed());

    // Wrap a different message with the existing signature
    let different_payload = Payload {
        nonce: Uint128::from(0u128),
        msg: to_binary("eyJpbnN0YW50aWF0ZV9jb250cmFjdF93aXRoX3NlbGZfYWRtaW4iOnsiY29kZV9pZCI6MTY4OCwiaW5zdGFudGlhdGVfbXNnIjp7fX19ICA=").unwrap(),
        expiration: None,
        contract_address: Addr::unchecked("contract_address").to_string(),
        bech32_prefix: "cosmos".to_string(),
        contract_version: "version-1".to_string(),
        chain_id: "juno-1".to_string(),
    };

    let wrapped_msg = WrappedMessage {
        payload: different_payload,
        signature: sig.serialize_compact().into(),
        public_key: hex_encoded.clone(),
    };

    let err: ContractError =
        verify(deps.as_mut(), env.clone(), &mut info, wrapped_msg).unwrap_err();

    assert!(matches!(err, ContractError::SignatureInvalid { .. }));
}

#[test]
fn test_verify_malformed_signature() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let mut info = mock_info("creator", &[]);

    let payload = Payload {
        nonce: Uint128::from(0u128),
        msg: to_binary("eyJpbnN0YW50aWF0ZV9jb250cmFjdF93aXRoX3NlbGZfYWRtaW4iOnsiY29kZV9pZCI6MTY4OCwiaW5zdGFudGlhdGVfbXNnIjp7fX19ICA=").unwrap(),
        expiration: None,
        contract_address: Addr::unchecked("contract_address").to_string(),
        bech32_prefix: JUNO_PREFIX.to_string(),
        contract_version: "version-1".to_string(),
        chain_id: "juno-1".to_string(),
    };

    let mut wrapped_msg = get_wrapped_msg(deps.as_mut(), payload);
    let malformed_sig = Vec::from("malformed signature");
    wrapped_msg.signature = malformed_sig.into();

    let err: ContractError =
        verify(deps.as_mut(), env.clone(), &mut info, wrapped_msg).unwrap_err();
    assert!(matches!(err, ContractError::VerificationError { .. }));
}

// Verify that sender's address is set correctly in info.
#[test]
fn test_verify_correct_address() {
    let payload = Payload {
        nonce: Uint128::from(0u128),
        msg: to_binary("test").unwrap(),
        expiration: None,
        contract_address: Addr::unchecked("contract_address").to_string(),
        bech32_prefix: "juno".to_string(),
        contract_version: "version-1".to_string(),
        chain_id: "juno-1".to_string(),
    };
    let mut deps = mock_dependencies();

    let wrapped_msg = get_wrapped_msg(deps.as_mut(), payload);

    let mut env = mock_env();
    env.block.height = 1;
    let mut info = mock_info("creator", &[]);
    verify(deps.as_mut(), env, &mut info, wrapped_msg.clone()).unwrap();

    let addr = pk_to_addr(deps.as_ref(), wrapped_msg.public_key.to_hex(), JUNO_PREFIX).unwrap();

    assert_eq!(info.sender, addr);
}

// Generate a validly signed message but without creating a sign doc first.
#[test]
fn test_verify_no_sign_doc() {
    let payload = Payload {
        nonce: Uint128::from(0u128),
        msg: to_binary("eyJpbnN0YW50aWF0ZV9jb250cmFjdF93aXRoX3NlbGZfYWRtaW4iOnsiY29kZV9pZCI6MTY4OCwiaW5zdGFudGlhdGVfbXNnIjp7fX19ICA=").unwrap(),
        expiration: None,
        contract_address: Addr::unchecked("contract_address").to_string(),
        bech32_prefix: "juno".to_string(),
        contract_version: "version-1".to_string(),
        chain_id: "juno-1".to_string(),
    };

    let mut deps = mock_dependencies();

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

    // Verify should fail
    let env = mock_env();
    let mut info = mock_info("creator", &[]);
    let res = verify(deps.as_mut(), env, &mut info, wrapped_msg.clone());
    assert!(matches!(res, Err(ContractError::SignatureInvalid { .. })));
}

// signs a given payload and returns the wrapped message
fn get_wrapped_msg(deps: DepsMut, payload: Payload) -> WrappedMessage {
    // Generate a keypair
    let secp = Secp256k1::new();
    let (secret_key, public_key) = secp.generate_keypair(&mut OsRng);

    // Generate signdoc
    let signer_addr = pk_to_addr(
        deps.as_ref(),
        public_key.to_hex(), // to_hex ensures that the public key has the expected number of bytes
        &payload.bech32_prefix,
    )
    .unwrap();

    let payload_ser = serde_json::to_string(&payload).unwrap();

    let sign_doc = get_sign_doc(signer_addr.as_str(), &payload_ser, &"juno-1").unwrap();

    // Hash and sign the payload
    let msg_hash = Sha256::digest(&to_binary(&sign_doc).unwrap());
    let msg = Message::from_slice(&msg_hash).unwrap();
    let sig = secp.sign_ecdsa(&msg, &secret_key);

    // Wrap the message
    let hex_encoded = HexBinary::from(public_key.serialize_uncompressed());
    WrappedMessage {
        payload,
        signature: sig.serialize_compact().into(),
        public_key: hex_encoded.clone(),
    }
}

/*
Moar tests to write:
wrong version
load a keypair corresponding to pre-known address and validate that address in info was set correctly
test integrating with another contract
wrong contract address
*/
