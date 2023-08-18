use cosmwasm_std::StdError;
use cw_utils::PaymentError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    PaymentError(#[from] PaymentError),

    #[error(transparent)]
    HookError(#[from] cw_hooks::HookError),

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

    #[error("Absolute count threshold cannot be greater than the total token supply")]
    InvalidAbsoluteCount {},

    #[error("Active threshold percentage must be greater than 0 and less than 1")]
    InvalidActivePercentage {},

    #[error("Can only unstake less than or equal to the amount you have staked")]
    InvalidUnstakeAmount {},

    #[error("Active threshold count must be greater than zero")]
    ZeroActiveCount {},

    #[error("Amount being unstaked must be non-zero")]
    ZeroUnstake {},
}
