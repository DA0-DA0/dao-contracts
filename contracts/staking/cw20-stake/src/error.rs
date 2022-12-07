use cosmwasm_std::{Addr, StdError};
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),
    #[error("{0}")]
    Cw20Error(#[from] cw20_base::ContractError),
    #[error("Nothing to claim")]
    NothingToClaim {},
    #[error("Nothing to unstake")]
    NothingStaked {},
    #[error("Unstaking this amount violates the invariant: (cw20 total_supply <= 2^128)")]
    Cw20InvaraintViolation {},
    #[error("Can not unstake more than has been staked")]
    ImpossibleUnstake {},
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
}
