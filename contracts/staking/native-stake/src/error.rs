use cosmwasm_std::{Addr, StdError};
use cw_utils::PaymentError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),
    #[error("{0}")]
    PaymentError(#[from] PaymentError),
    #[error("Nothing to claim")]
    NothingToClaim {},
    #[error("Invalid token")]
    InvalidToken { received: Addr, expected: Addr },
    #[error("Unauthorized")]
    Unauthorized {},
    #[error("Too many outstanding claims. Claim some tokens before unstaking more.")]
    TooManyClaims {},
    #[error("No admin configured")]
    NoAdminConfigured {},
    #[error("{0}")]
    HookError(#[from] cw_controllers::HookError),
    #[error("Only owner can change owner")]
    OnlyOwnerCanChangeOwner {},
    #[error("Invalid unstaking duration, unstaking duration cannot be 0")]
    InvalidUnstakingDuration {},
    #[error("Can only unstake less than or equal to the amount you have staked")]
    InvalidUnstakeAmount {},
}
