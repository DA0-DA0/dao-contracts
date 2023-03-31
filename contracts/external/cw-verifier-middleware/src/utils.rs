use cosmwasm_std::{to_binary, Api, HexBinary};
use secp256k1::{hashes::hex::ToHex, rand::rngs::OsRng, Message, Secp256k1};
use sha2::{Digest, Sha256};

use crate::{
    msg::{Payload, WrappedMessage},
    verify::{get_sign_doc, pk_to_addr},
};

// signs a given payload and returns the wrapped message
pub fn get_wrapped_msg(api: &dyn Api, payload: Payload) -> WrappedMessage {
    // Generate a keypair
    let secp = Secp256k1::new();
    let (secret_key, public_key) = secp.generate_keypair(&mut OsRng);

    // Generate signdoc
    let signer_addr = pk_to_addr(
        api,
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
