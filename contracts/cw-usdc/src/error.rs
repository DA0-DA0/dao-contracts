use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    // #[error("Custom Error val: {val:?}")]
    // CustomError { val: String },
    // // Add any other custom errors you like here.
    // // Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.
    #[error("Invalid subdenom: {subdenom:?}")]
    InvalidSubdenom { subdenom: String },

    #[error("Invalid denom: {denom:?} {message:?}")]
    InvalidDenom { denom: String, message: String },

    #[error("denom does not exist: {denom:?}")]
    DenomDoesNotExist { denom: String },

    #[error("Not enough {denom:?} ({funds:?}) in funds. {needed:?} {denom:?} needed")]
    NotEnoughFunds {denom: String, funds: u128, needed: u128},

    #[error("address is not supported yet, was: {address:?}")]
    BurnFromAddressNotSupported { address: String },

    #[error("amount was zero, must be positive")]
    ZeroAmount {},

    #[error("The address '{address}' is blacklisted")]
    Blacklisted { address: String },

    #[error("The contract is frozen for denom {denom:?}")]
    ContractFrozen { denom: String },

    #[error("Frozen status is already {status:?}")]
    ContractFrozenStatusUnchanged { status: bool },

    #[error("Freezer status is already {status:?}")]
    FreezerStatusUnchanged { status: bool },
}
