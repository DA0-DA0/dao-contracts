use cosmwasm_std::{Addr, StdError};
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error(transparent)]
    Cw20Error(#[from] cw20_base::ContractError),

    #[error(transparent)]
    Ownership(#[from] cw_ownable::OwnershipError),

    #[error(transparent)]
    HookError(#[from] cw_hooks::HookError),

    #[error(transparent)]
    UnstakingDurationError(#[from] dao_voting::duration::UnstakingDurationError),

    #[error("can not migrate. current version is up to date")]
    AlreadyMigrated {},

    #[error("Unstaking this amount violates the invariant: (cw20 total_supply <= 2^128)")]
    Cw20InvaraintViolation {},

    #[error("Can not unstake more than has been staked")]
    ImpossibleUnstake {},

    #[error("Provided cw20 errored in response to TokenInfo query")]
    InvalidCw20 {},

    #[error("Invalid token")]
    InvalidToken { received: Addr, expected: Addr },

    #[error("Nothing to claim")]
    NothingToClaim {},

    #[error("Nothing to unstake")]
    NothingStaked {},

    #[error("Too many outstanding claims. Claim some tokens before unstaking more.")]
    TooManyClaims {},
}
