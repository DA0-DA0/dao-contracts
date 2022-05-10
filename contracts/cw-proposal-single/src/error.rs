use std::u64;

use cosmwasm_std::StdError;
use indexable_hooks::HookError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    HookError(#[from] HookError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("{0}")]
    ThresholdError(#[from] voting::ThresholdError),

    #[error("Suggested proposal expiration is larger than the maximum proposal duration")]
    InvalidExpiration {},

    #[error("No such proposal ({id})")]
    NoSuchProposal { id: u64 },

    #[error("Proposal is ({size}) bytes, must be <= ({max}) bytes")]
    ProposalTooLarge { size: u64, max: u64 },

    #[error("Proposal is not open ({id})")]
    NotOpen { id: u64 },

    #[error("Proposal is expired ({id})")]
    Expired { id: u64 },

    #[error("Not registered to vote (no voting power) at time of proposal creation.")]
    NotRegistered {},

    #[error("Already voted")]
    AlreadyVoted {},

    #[error("Proposal is not passed.")]
    NotPassed {},

    #[error("Proposal has already been executed.")]
    AlreadyExecuted {},

    #[error("Proposal is closed.")]
    Closed {},

    #[error("Only rejected or expired proposals may be closed.")]
    WrongCloseStatus {},

    #[error("The DAO is currently inactive, you cannot create proposals")]
    InactiveDao {},
}
