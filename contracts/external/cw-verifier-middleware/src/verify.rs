use cosmwasm_std::{Binary, Timestamp, to_binary, DepsMut, Env, Addr, MessageInfo, Uint128, StdError, OverflowError
};
use sha2::{Sha256, Digest};
use crate::{error::ContractError, state::NONCE};
use secp256k1::{Message as SecpMessage, PublicKey, Secp256k1, ecdsa::Signature,};
use cosmwasm_schema::{cw_serde};

pub fn verify(deps: DepsMut, env: Env, mut info: MessageInfo, wrapped_msg: WrappedMessage) -> Result<Binary, ContractError>{
    let secp = Secp256k1::verification_only();

    // Serialize the inner message
    let msg_ser = to_binary(&wrapped_msg.payload)?;

    // Hash the serialized payload using SHA-256
    let msg_hash = Sha256::digest(&msg_ser);

    // Verify the signature
    let msg_secp = SecpMessage::from_slice(&msg_hash)?;
    let public_key = PublicKey::from_slice(&wrapped_msg.public_key).unwrap();
    let signature = Signature::from_der(&wrapped_msg.signature).unwrap();
    secp.verify_ecdsa(&msg_secp, &signature, &public_key)?;

    // Validate that the message has the correct nonce
    let nonce = NONCE.load(deps.storage)?;
    if wrapped_msg.payload.nonce != nonce.u128() {
        return Err(ContractError::InvalidNonce { });
    }

    // Increment nonce 
    NONCE.update(deps.storage, |nonce| nonce.checked_add(Uint128::from(1u128)).map_err(|e| StdError::from(e)))?;

    // Validate that the message has not expired
    if let Some(expiration) = wrapped_msg.payload.expiration {
        if expiration < env.block.time {
            return Err(ContractError::MessageExpired { });
        }
    }

    // Set the message sender to the address corresponding to the provided public key. (pk_to_addr)
    let sender = pk_to_addr(wrapped_msg.public_key);
    info.sender = sender;

    // Return the msg; caller will deserialize
    return Ok(wrapped_msg.payload.msg)
}

#[cw_serde]
pub struct WrappedMessage {
    pub payload: Payload,
    pub signature: Binary, 
    pub public_key: Binary,
}

#[cw_serde]
pub struct Payload {
    pub nonce: u128,
    pub msg: Binary,
    pub expiration: Option<Timestamp>,
}

// mock pk_to_addr
pub fn pk_to_addr(_pk: Binary) -> Addr {
    return Addr::unchecked("dummy_addr")
}

mod tests {
    use cosmwasm_std::{Binary, to_binary,testing::{mock_dependencies, mock_env, mock_info},};
    use secp256k1::{SecretKey, ffi::PublicKey, rand::{self, rngs::OsRng}, Secp256k1, Message};
    use sha2::{Sha256, Digest};

    use crate::verify::verify;

    use super::{Payload, WrappedMessage};

    #[test]
    fn test_verify_signature() {
        let payload = Payload {
            nonce: 0,
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
}