use cosmwasm_std::StdError;
use cw_utils::ParseReplyError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("StorageError")]
    StorageError {},

    #[error("Unauthorized {reason:?}")]
    Unauthorized { reason: Option<String> },

    #[error("InvalidMessage")]
    InvalidMessageError {},

    #[error("An unknown reply ID was received.")]
    UnknownReplyID {},

    #[error("{0}")]
    ParseReplyError(#[from] ParseReplyError),

    #[error("MultipleParents: already proxying a proposal")]
    MultipleParents {},
}
