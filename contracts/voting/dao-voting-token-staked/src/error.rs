use cosmwasm_std::{StdError, Uint128};
use cw_utils::{ParseReplyError, PaymentError};
use dao_voting::threshold::ActiveThresholdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error(transparent)]
    ActiveThresholdError(#[from] ActiveThresholdError),

    #[error(transparent)]
    HookError(#[from] cw_hooks::HookError),

    #[error(transparent)]
    PaymentError(#[from] PaymentError),

    #[error(transparent)]
    ParseReplyError(#[from] ParseReplyError),

    #[error(transparent)]
    UnstakingDurationError(#[from] dao_voting::duration::UnstakingDurationError),

    #[error("Initial governance token balances must not be empty")]
    InitialBalancesError {},

    #[error("Can only unstake less than or equal to the amount you have staked")]
    InvalidUnstakeAmount {},

    #[error("Nothing to claim")]
    NothingToClaim {},

    #[error("Too many outstanding claims. Claim some tokens before unstaking more.")]
    TooManyClaims {},

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Got a submessage reply with unknown id: {id}")]
    UnknownReplyId { id: u64 },

    #[error("Amount being unstaked must be non-zero")]
    ZeroUnstake {},

    #[error("Limit cannot be exceeded")]
    LimitExceeded { limit: Uint128 },
}
