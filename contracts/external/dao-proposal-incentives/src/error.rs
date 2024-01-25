use cosmwasm_std::{StdError, Uint128};
use cw_utils::{ParseReplyError, PaymentError};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    PaymentError(#[from] PaymentError),

    #[error("You need to deposit enough incentives for at least one epoch of incentives. Expected {expected}, got {actual}.")]
    InsufficientInitialDeposit { expected: Uint128, actual: Uint128 },

    #[error("{0}")]
    ParseReplyError(#[from] ParseReplyError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("An unknown reply ID was received.")]
    UnknownReplyID {},
}
