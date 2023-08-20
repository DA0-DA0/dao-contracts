use cosmwasm_std::StdError;
use cw_utils::{ParseReplyError, PaymentError};
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error(transparent)]
    PaymentError(#[from] PaymentError),

    #[error(transparent)]
    ParseReplyError(#[from] ParseReplyError),

    #[error(transparent)]
    HookError(#[from] cw_hooks::HookError),

    #[error("Absolute count threshold cannot be greater than the total token supply")]
    InvalidAbsoluteCount {},

    #[error("Active threshold percentage must be greater than 0 and less than 1")]
    InvalidActivePercentage {},

    #[error("Initial governance token balances must not be empty")]
    InitialBalancesError {},

    #[error("Can only unstake less than or equal to the amount you have staked")]
    InvalidUnstakeAmount {},

    #[error("Invalid unstaking duration, unstaking duration cannot be 0")]
    InvalidUnstakingDuration {},

    #[error("Only owner can change owner")]
    OnlyOwnerCanChangeOwner {},

    #[error("Nothing to claim")]
    NothingToClaim {},

    #[error("Too many outstanding claims. Claim some tokens before unstaking more.")]
    TooManyClaims {},

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Got a submessage reply with unknown id: {id}")]
    UnknownReplyId { id: u64 },

    #[error("Active threshold count must be greater than zero")]
    ZeroActiveCount {},

    #[error("Amount being unstaked must be non-zero")]
    ZeroUnstake {},
}
