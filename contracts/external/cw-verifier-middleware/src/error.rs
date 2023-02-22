use cosmwasm_std::{OverflowError, StdError, VerificationError};
use hex::FromHexError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    FromHexError(#[from] FromHexError),

    #[error("{0}")]
    VerificationError(#[from] VerificationError),

    #[error("Invalid nonce")]
    InvalidNonce,

    #[error("Message expiration has passed")]
    MessageExpired,

    #[error("Message signature is invalid")]
    SignatureInvalid,

    #[error("Invalid uncompressed public key hex string length; expected 130 bytes, got {length}")]
    InvalidPublicKeyLength { length: usize },
}
