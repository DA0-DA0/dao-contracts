
use cosmwasm_std::{StdError, OverflowError};
use thiserror::Error;
use secp256k1::Error as SecpError;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    OverflowError(#[from] OverflowError),

    #[error("{0}")]
    Secp256k1Error(#[from] SecpError),

    #[error("Invalid nonce")]
    InvalidNonce,

    #[error("Message expiration has passed")]
    MessageExpired,
}


