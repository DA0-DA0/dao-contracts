use cosmwasm_std::{to_binary, Uint128, testing::{mock_dependencies, mock_env, mock_info}};
use secp256k1::{Secp256k1, Message, rand::rngs::OsRng};
use sha2::{Sha256, Digest};

use crate::{verify::{ec_pk_to_bech32_address, ADDR_PREFIX, verify}, msg::{Payload, WrappedMessage}};

#[test]
fn test_generate_juno_addr_from_pk() {

    let juno_address = "juno1muw4rz9ml44wc6vssqrzkys4nuc3gylrxj4flw".to_string();
    let juno_pk = "04f620cd2e33d3f6af5a43d5b3ca3b9b7f653aa980ae56714cc5eb7637fd1eeb28fb722c0dacb5f005f583630dae8bbe7f5eaba70f129fc279d7ff421ae8c9eb79".to_string();

    let generated_address = ec_pk_to_bech32_address(
        juno_pk,
        ADDR_PREFIX,
    ).unwrap();
    assert_eq!(generated_address, juno_address);
}

#[test]
fn test_verify() {
    let payload = Payload {
        nonce: Uint128::from(0u128),
        msg: to_binary("eyJpbnN0YW50aWF0ZV9jb250cmFjdF93aXRoX3NlbGZfYWRtaW4iOnsiY29kZV9pZCI6MTY4OCwiaW5zdGFudGlhdGVfbXNnIjoiZXlKaFpHMXBiaUk2Ym5Wc2JDd2lZWFYwYjIxaGRHbGpZV3hzZVY5aFpHUmZZM2N5TUhNaU9uUnlkV1VzSW1GMWRHOXRZWFJwWTJGc2JIbGZZV1JrWDJOM056SXhjeUk2ZEhKMVpTd2laR1Z6WTNKcGNIUnBiMjRpT2lJeElpd2lhVzFoWjJWZmRYSnNJanB1ZFd4c0xDSnVZVzFsSWpvaWRHVnpkQ0lzSW5CeWIzQnZjMkZzWDIxdlpIVnNaWE5mYVc1emRHRnVkR2xoZEdWZmFXNW1ieUk2VzNzaVlXUnRhVzRpT25zaVkyOXlaVjl0YjJSMWJHVWlPbnQ5ZlN3aVkyOWtaVjlwWkNJNk1UWTVOQ3dpYkdGaVpXd2lPaUpFUVU5ZmRHVnpkRjlFWVc5UWNtOXdiM05oYkZOcGJtZHNaU0lzSW0xelp5STZJbVY1U21oaVIzaDJaREU1ZVZwWVduWmtSMngxV25sSk5scHRSbk5qTWxWelNXMU9jMkl6VG14WU0wSjVZak5DZG1NeVJuTllNamwxV0RKV05GcFhUakZrUjJ4MlltdzViVmxYYkhOa1dFcHNTV3B3TUdOdVZteE1RMHAwV1Zob1ptUnRPVEJoVnpWdVdETkNiR050YkhaYVEwazJaWGxLTUdGWE1XeEphbTh5VFVSUk5FMUVRamxNUTBwMFlWYzFabVJ0T1RCaFZ6VnVXRE5DYkdOdGJIWmFRMGsyWW01V2MySkRkMmxpTWpWelpWWTVkRnBYTVdsYVdFcDZXREpXTkZwWFRqRmtSMVZwVDI1U2VXUlhWWE5KYmtKNVdsWTVkMk50T1hkaU0wNXNXREpzZFZwdE9HbFBibk5wWWxjNWEyUlhlR3hZTWpGb1pWWTVkMk50T1hkaU0wNXNTV3B3TjBsdGJIVmFiVGhwVDI1emFWbFhVblJoVnpScFQyNXphVmt5T1hsYVZqbDBZakpTTVdKSFZXbFBiblE1WmxOM2FWa3lPV3RhVmpsd1drTkpOazFVV1RWTmFYZHBZa2RHYVZwWGQybFBhVXBGVVZVNVptUkhWbnBrUmpsM1kyMVZkR05JU25aalJ6bDZXbE14UlZsWE9WRmpiVGwzWWpOT2FHSkdUbkJpYldSeldsTkpjMGx0TVhwYWVVazJTVzFXTlZOdGRHRlhSVW95V1hwS2MwMUdaM2xpU0ZaaFlsUm9jRlF5TURGTlYwcElaRE5PU21KV1dUQmFSV1JYWkZkTmVXSklXbWxoVldzeVdsUk5kMk13YkhSUFdHUmhWbnBXYlZrd2FFdGtiVTVJVDFod1dsWXphRzFaZWs1WFlWZEtXR0pJY0dwTmJYZ3lXVzFzU2s1c2NIUlNiazVxVFd4Wk5VbHVNVGxtVTNkcFpFZG9lVnBZVG05aU1uaHJTV3B3TjBsdVVtOWpiVlo2WVVjNWMxcEdPWGhrVnpsNVpGY3dhVTl1YzJsaldGWjJZMjVXZEVscWNEZEpia0pzWTIxT2JHSnVVV2xQYVVsM1RHcEpkMGx1TUhOSmJsSnZZMjFXZW1GSE9YTmFRMGsyWlhsS2RGbFhjSFpqYld3d1pWTkpObVV6TVRsbVdERTVJbjFkTENKMmIzUnBibWRmYlc5a2RXeGxYMmx1YzNSaGJuUnBZWFJsWDJsdVptOGlPbnNpWVdSdGFXNGlPbnNpWTI5eVpWOXRiMlIxYkdVaU9udDlmU3dpWTI5a1pWOXBaQ0k2TVRZNU5pd2liR0ZpWld3aU9pSkVRVTlmZEdWemRGOUVZVzlXYjNScGJtZERkelFpTENKdGMyY2lPaUpsZVVwcVpIcFNabG96U25aa1dFSm1XVEk1YTFwV09YQmFRMGsyVFZSWk1rOURkMmxoVnpWd1pFZHNhR0pHT1hSYVZ6RnBXbGhLZWtscWNHSmxlVXBvV2tkU2VVbHFiMmxoYmxaMVlucEdNbU5ZYURKbFdHTXlZVE5DTlU0emFIRk5SekY2WlVodk1VNHpUakprTWpRd1kwUkNjbHB0VWpGT1JGcHlaR3BDZDJGNVNYTkpibVJzWVZka2IyUkRTVFpOV0RGa1psRTlQU0o5ZlE9PSIsImxhYmVsIjoidGVzdCJ9fQ==").unwrap(),
        expiration: None,
    };

    let secp = Secp256k1::new();
    let (secret_key, public_key) = secp.generate_keypair(&mut OsRng);
    let msg_hash = Sha256::digest(&to_binary(&payload).unwrap());
    let msg = Message::from_slice(&msg_hash).unwrap();
    let sig = secp.sign_ecdsa(&msg, &secret_key);

    let wrapped_msg = WrappedMessage {
        payload,
        signature: sig.serialize_compact().into(),
        public_key: public_key.serialize().into(),
    };

    let mut deps = mock_dependencies();
    let env = mock_env(); 
    let info = mock_info("creator", &[]);
    assert!(verify(deps.as_mut(), env, info, wrapped_msg).is_ok());
}