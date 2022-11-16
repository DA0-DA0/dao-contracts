use cosmwasm_std::StdError;
use thiserror::Error;

use cosmwasm_std::Addr;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Not authorized to perform action")]
    Unauthorized {},

    #[error("The start time is invalid. Start time must be before the end time and after the current block time")]
    InvalidStartTime {},

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

    #[error("Numerical overflow")]
    Overflow {},
}
