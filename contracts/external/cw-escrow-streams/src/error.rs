use cosmwasm_std::{Addr, StdError, Uint128};
use cw_denom::DenomError;
use thiserror::Error;

use crate::state::StreamId;

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

    #[error("The stream has been fully claimed")]
    StreamFullyClaimed {},

    #[error("The stream can only be claimed by original recipient")]
    NotStreamRecipient { recipient: Addr },

    #[error("No tokens have vested for this stream")]
    NoFundsToClaim { claimed: Uint128 },

    #[error("No funds attached")]
    NoFundsAttached {},

    #[error("Stream does not exist")]
    StreamNotFound { stream_id: StreamId },

    #[error("Stream recipient cannot be the stream owner")]
    InvalidRecipient {},

    #[error("Can not pause paused stream")]
    StreamAlreadyPaused {},

    #[error("Stream is not pause for resume")]
    StreamNotPaused {},

    #[error("Could not create bank transfer message")]
    CouldNotCreateBankMessage {},

    #[error("Left and right stream should not be equal to each other")]
    StreamsShouldNotBeEqual {},

    #[error("Invalid Stream Ids")]
    InvalidStreamIds {},

    #[error("Stream is not linked")]
    StreamNotLinked {},

    #[error("Stream is not detachable")]
    StreamNotDetachable {},

    #[error("Stream can't be deleted, as it's linked to another stream")]
    LinkedStreamDeleteNotAllowed { link_id: StreamId },
}
