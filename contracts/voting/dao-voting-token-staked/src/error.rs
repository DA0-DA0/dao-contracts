use cosmwasm_std::StdError;
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

    #[error("Factory contract did not implment the required TokenFactoryCallback interface")]
    NoFactoryCallback {},

    #[error("Nothing to claim")]
    NothingToClaim {},

    #[error("Too many outstanding claims. Claim some tokens before unstaking more.")]
    TooManyClaims {},

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Got a submessage reply with unknown id: {id}")]
    UnknownReplyId { id: u64 },

    #[error("Factory message must serialize to WasmMsg::Execute")]
    UnsupportedFactoryMsg {},

    #[error("Amount being unstaked must be non-zero")]
    ZeroUnstake {},
}
