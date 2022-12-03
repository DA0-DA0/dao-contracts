use cosmwasm_std::{StdError, Uint128};
use thiserror::Error;

use cosmwasm_std::Addr;

#[derive(Error, Debug, PartialEq)]
pub enum GenericError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("No coin balance found")]
    EmptyBalance {},

    #[error("Not enough cw20 balance of {addr}, need {lack} more")]
    NotEnoughCw20 { addr: String, lack: Uint128 },

    #[error("Not enough native balance of {denom}, need {lack} more")]
    NotEnoughNative { denom: String, lack: Uint128 },

    #[error("invalid cosmwasm message")]
    InvalidWasmMsg {},

    #[error("Numerical overflow")]
    IntegerOverflow {},
}


#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Not authorized to perform action")]
    Unauthorized {},

    #[error("The start time is invalid. Start time must be before the end time and after the current block time")]
    InvalidStartTime {},

    #[error("The end time is invalid. End time must be before current block time")]
    InvalidEndTime {},

    #[error("The stream has been fully claimed")]
    StreamFullyClaimed {},

    #[error("The stream can only be claimed by original recipient")]
    NotStreamRecipient { recipient: Addr },

    #[error("No tokens have vested for this stream.")]
    NoFundsToClaim {},

    #[error("Stream does not exist.")]
    StreamNotFound {},

    #[error("Amount must be greater than duration")]
    AmountLessThanDuration {},

    #[error("Stream recipient cannot be the stream owner")]
    InvalidRecipient {},

    #[error("Can not pause paused stream.")]
    StreamAlreadyPaused {},

    #[error("Stream is not pause for resume!")]
    StreamNotPaused {},
}
