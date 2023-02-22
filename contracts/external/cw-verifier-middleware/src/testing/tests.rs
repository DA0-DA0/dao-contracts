use cosmwasm_std::{
    testing::{mock_dependencies, mock_env, mock_info},
    to_binary, Addr, HexBinary, Uint128,
};
use secp256k1::{rand::rngs::OsRng, Message, Secp256k1};
use sha2::{Digest, Sha256};

use crate::{
    msg::{Payload, WrappedMessage},
    verify::{pk_to_addr, verify},
};

#[test]
fn test_pk_to_addr() {
    let juno_address = Addr::unchecked("juno1muw4rz9ml44wc6vssqrzkys4nuc3gylrxj4flw");
    let juno_pk = "04f620cd2e33d3f6af5a43d5b3ca3b9b7f653aa980ae56714cc5eb7637fd1eeb28fb722c0dacb5f005f583630dae8bbe7f5eaba70f129fc279d7ff421ae8c9eb79".to_string();

    let generated_address = pk_to_addr(juno_pk, "juno").unwrap();
    assert_eq!(generated_address, juno_address);
}

#[test]
fn test_verify_success() {
    // This test generates a payload in which the signature is of base64 format, and the public key is of hex format.
    // The test then calls verify to validate that the signature is correctly verified.

    let payload = Payload {
        nonce: Uint128::from(0u128),
        msg: to_binary("eyJpbnN0YW50aWF0ZV9jb250cmFjdF93aXRoX3NlbGZfYWRtaW4iOnsiY29kZV9pZCI6MTY4OCwiaW5zdGFudGlhdGVfbXNnIjoiZXlKaFpHMXBiaUk2Ym5Wc2JDd2lZWFYwYjIxaGRHbGpZV3hzZVY5aFpHUmZZM2N5TUhNaU9uUnlkV1VzSW1GMWRHOXRZWFJwWTJGc2JIbGZZV1JrWDJOM056SXhjeUk2ZEhKMVpTd2laR1Z6WTNKcGNIUnBiMjRpT2lJeElpd2lhVzFoWjJWZmRYSnNJanB1ZFd4c0xDSnVZVzFsSWpvaWRHVnpkQ0lzSW5CeWIzQnZjMkZzWDIxdlpIVnNaWE5mYVc1emRHRnVkR2xoZEdWZmFXNW1ieUk2VzNzaVlXUnRhVzRpT25zaVkyOXlaVjl0YjJSMWJHVWlPbnQ5ZlN3aVkyOWtaVjlwWkNJNk1UWTVOQ3dpYkdGaVpXd2lPaUpFUVU5ZmRHVnpkRjlFWVc5UWNtOXdiM05oYkZOcGJtZHNaU0lzSW0xelp5STZJbVY1U21oaVIzaDJaREU1ZVZwWVduWmtSMngxV25sSk5scHRSbk5qTWxWelNXMU9jMkl6VG14WU0wSjVZak5DZG1NeVJuTllNamwxV0RKV05GcFhUakZrUjJ4MlltdzViVmxYYkhOa1dFcHNTV3B3TUdOdVZteE1RMHAwV1Zob1ptUnRPVEJoVnpWdVdETkNiR050YkhaYVEwazJaWGxLTUdGWE1XeEphbTh5VFVSUk5FMUVRamxNUTBwMFlWYzFabVJ0T1RCaFZ6VnVXRE5DYkdOdGJIWmFRMGsyWW01V2MySkRkMmxpTWpWelpWWTVkRnBYTVdsYVdFcDZXREpXTkZwWFRqRmtSMVZwVDI1U2VXUlhWWE5KYmtKNVdsWTVkMk50T1hkaU0wNXNXREpzZFZwdE9HbFBibk5wWWxjNWEyUlhlR3hZTWpGb1pWWTVkMk50T1hkaU0wNXNTV3B3TjBsdGJIVmFiVGhwVDI1emFWbFhVblJoVnpScFQyNXphVmt5T1hsYVZqbDBZakpTTVdKSFZXbFBiblE1WmxOM2FWa3lPV3RhVmpsd1drTkpOazFVV1RWTmFYZHBZa2RHYVZwWGQybFBhVXBGVVZVNVptUkhWbnBrUmpsM1kyMVZkR05JU25aalJ6bDZXbE14UlZsWE9WRmpiVGwzWWpOT2FHSkdUbkJpYldSeldsTkpjMGx0TVhwYWVVazJTVzFXTlZOdGRHRlhSVW95V1hwS2MwMUdaM2xpU0ZaaFlsUm9jRlF5TURGTlYwcElaRE5PU21KV1dUQmFSV1JYWkZkTmVXSklXbWxoVldzeVdsUk5kMk13YkhSUFdHUmhWbnBXYlZrd2FFdGtiVTVJVDFod1dsWXphRzFaZWs1WFlWZEtXR0pJY0dwTmJYZ3lXVzFzU2s1c2NIUlNiazVxVFd4Wk5VbHVNVGxtVTNkcFpFZG9lVnBZVG05aU1uaHJTV3B3TjBsdVVtOWpiVlo2WVVjNWMxcEdPWGhrVnpsNVpGY3dhVTl1YzJsaldGWjJZMjVXZEVscWNEZEpia0pzWTIxT2JHSnVVV2xQYVVsM1RHcEpkMGx1TUhOSmJsSnZZMjFXZW1GSE9YTmFRMGsyWlhsS2RGbFhjSFpqYld3d1pWTkpObVV6TVRsbVdERTVJbjFkTENKMmIzUnBibWRmYlc5a2RXeGxYMmx1YzNSaGJuUnBZWFJsWDJsdVptOGlPbnNpWVdSdGFXNGlPbnNpWTI5eVpWOXRiMlIxYkdVaU9udDlmU3dpWTI5a1pWOXBaQ0k2TVRZNU5pd2liR0ZpWld3aU9pSkVRVTlmZEdWemRGOUVZVzlXYjNScGJtZERkelFpTENKdGMyY2lPaUpsZVVwcVpIcFNabG96U25aa1dFSm1XVEk1YTFwV09YQmFRMGsyVFZSWk1rOURkMmxoVnpWd1pFZHNhR0pHT1hSYVZ6RnBXbGhLZWtscWNHSmxlVXBvV2tkU2VVbHFiMmxoYmxaMVlucEdNbU5ZYURKbFdHTXlZVE5DTlU0emFIRk5SekY2WlVodk1VNHpUakprTWpRd1kwUkNjbHB0VWpGT1JGcHlaR3BDZDJGNVNYTkpibVJzWVZka2IyUkRTVFpOV0RGa1psRTlQU0o5ZlE9PSIsImxhYmVsIjoidGVzdCJ9fQ==").unwrap(),
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
        public_key: hex_encoded,
    };

    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info("creator", &[]);
    verify(deps.as_mut(), env, info, wrapped_msg).unwrap();
}
