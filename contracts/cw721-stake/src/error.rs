use cosmwasm_std::{Addr, StdError};
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),
    #[error("Nothing to claim")]
    NothingToClaim {},
    #[error("No reward token to fund")]
    NothingToFund {},
    #[error("Invalid token")]
    InvalidToken { received: Addr, expected: Addr },
    #[error("Invalid address")]
    InvalidAddress {},
    #[error("Unauthorized")]
    Unauthorized {},
    #[error("Can not unstake that which you have not staked")]
    NotStaked {},
    #[error("Too many outstanding claims. Claim some tokens before unstaking more.")]
    TooManyClaims {},
    #[error("No admin configured")]
    NoAdminConfigured {},
    #[error("{0}")]
    HookError(#[from] cw_controllers::HookError),
    #[error("Only owner can change owner")]
    OnlyOwnerCanChangeOwner {},
}
