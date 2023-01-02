use cosmwasm_std::{Addr, StdError, Uint128};
use cw_denom::DenomError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error(transparent)]
    Denom(#[from] DenomError),

    #[error("Not authorized to perform action")]
    Unauthorized {},

    #[error("The start time is invalid. Start time must be before the end time and after the current block time")]
    InvalidStartTime {},

    #[error("The end time is invalid. End time must be before current block time")]
    InvalidEndTime {},

    #[error("No tokens have vested for this stream")]
    NoFundsToClaim { claimed: Uint128 },

    #[error("Stream does not exist")]
    StreamNotFound { stream_id: u64 },

    #[error("Stream recipient cannot be the stream owner")]
    InvalidRecipient {},

    #[error("Can not pause paused stream")]
    AlreadyPaused {},

    #[error("Stream is not pause for resume")]
    NotPaused {},

    #[error("Could not create bank transfer message")]
    CouldNotCreateBankMessage {},
}
