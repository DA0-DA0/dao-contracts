use cosmwasm_std::{Addr, StdError};
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Nothing to claim")]
    NothingToClaim {},

    #[error("Invalid token")]
    InvalidToken { received: Addr, expected: Addr },

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Can not unstake that which you have not staked")]
    NotStaked {},

    #[error("Can not stake that which has already been staked")]
    AlreadyStaked {},

    #[error("Too many outstanding claims. Claim some tokens before unstaking more.")]
    TooManyClaims {},

    #[error("{0}")]
    HookError(#[from] cw_controllers::HookError),

    #[error("Only owner can change owner")]
    OnlyOwnerCanChangeOwner {},

    #[error("Can't unstake zero NFTs.")]
    ZeroUnstake {},
}
