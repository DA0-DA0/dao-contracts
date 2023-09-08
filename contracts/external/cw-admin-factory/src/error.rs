use cosmwasm_std::{CoinsError, StdError};
use cw_ownable::OwnershipError;
use cw_utils::ParseReplyError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Coins(#[from] CoinsError),

    #[error("{0}")]
    Ownership(#[from] OwnershipError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("{0}")]
    ParseReplyError(#[from] ParseReplyError),

    #[error("An unknown reply ID was received.")]
    UnknownReplyID {},
}
