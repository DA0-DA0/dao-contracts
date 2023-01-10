use cosmwasm_std::StdError;
use cw_ownable::OwnershipError;
use cw_utils::{ParseReplyError, PaymentError};
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error(transparent)]
    Ownable(#[from] OwnershipError),

    #[error("{0}")]
    PaymentError(#[from] PaymentError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("{0}")]
    ParseReplyError(#[from] ParseReplyError),

    #[error("An unknown reply ID was received.")]
    UnknownReplyID {},
}
