use bech32::Error as Bech32Error;
use cosmwasm_std::{StdError, VerificationError};
use hex::FromHexError;
use secp256k1::Error as Secp256k1Error;
use serde_json::Error as SerdeError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    FromHexError(#[from] FromHexError),

    #[error("{0}")]
    VerificationError(#[from] VerificationError),

    #[error("{0}")]
    Bech32Error(#[from] Bech32Error),

    #[error("{0}")]
    Secp256k1Error(#[from] Secp256k1Error),

    #[error("{0}")]
    SerdeError(#[from] SerdeError),

    #[error("Invalid nonce")]
    InvalidNonce,

    #[error("Message expiration has passed")]
    MessageExpired,

    #[error("Message signature is invalid")]
    SignatureInvalid,

    #[error("Invalid uncompressed public key hex string length; expected 130 bytes, got {length}")]
    InvalidPublicKeyLength { length: usize },
}
