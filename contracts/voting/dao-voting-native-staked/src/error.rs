use cosmwasm_std::StdError;
use cw_utils::{ParseReplyError, PaymentError};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    PaymentError(#[from] PaymentError),

    #[error("{0}")]
    ParseReplyError(#[from] ParseReplyError),

    #[error("Got a submessage reply with unknown id: {id}")]
    UnknownReplyId { id: u64 },

    #[error("Error creating token")]
    TokenCreationError {},

    #[error("Initial governance token balances must not be empty")]
    InitialBalancesError {},

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Invalid unstaking duration, unstaking duration cannot be 0")]
    InvalidUnstakingDuration {},

    #[error("Nothing to claim")]
    NothingToClaim {},

    #[error("Too many outstanding claims. Claim some tokens before unstaking more.")]
    TooManyClaims {},

    #[error("Only owner can change owner")]
    OnlyOwnerCanChangeOwner {},

    #[error("Can only unstake less than or equal to the amount you have staked")]
    InvalidUnstakeAmount {},

    #[error("Amount being unstaked must be non-zero")]
    ZeroUnstake {},
}
