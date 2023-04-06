use cosmwasm_std::{Addr, StdError};
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error("Nothing to claim")]
    NothingToClaim {},

    #[error("Invalid token. Got ({received}), expected ({expected})")]
    InvalidToken { received: Addr, expected: Addr },

    #[error("Only the owner of this contract my execute this message")]
    NotOwner {},

    #[error("Can not unstake that which you have not staked (unstaking {token_id})")]
    NotStaked { token_id: String },

    #[error("Can not stake that which has already been staked")]
    AlreadyStaked {},

    #[error("Too many outstanding claims. Claim some tokens before unstaking more.")]
    TooManyClaims {},

    #[error(transparent)]
    HookError(#[from] cw_controllers::HookError),

    #[error("Can't unstake zero NFTs.")]
    ZeroUnstake {},
}
