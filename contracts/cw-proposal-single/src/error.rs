use std::u64;

use cosmwasm_std::StdError;
use cw_utils::ParseReplyError;
use indexable_hooks::HookError;
use thiserror::Error;
use voting::reply::error::TagError;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error(transparent)]
    ParseReplyError(#[from] ParseReplyError),

    #[error(transparent)]
    HookError(#[from] HookError),

    #[error("unauthorized")]
    Unauthorized {},

    #[error(transparent)]
    ThresholdError(#[from] voting::threshold::ThresholdError),

    #[error(transparent)]
    VotingError(#[from] voting::error::VotingError),

    #[error("no such proposal ({id})")]
    NoSuchProposal { id: u64 },

    #[error("proposal is ({size}) bytes, must be <= ({max}) bytes")]
    ProposalTooLarge { size: u64, max: u64 },

    #[error("proposal is not open ({id})")]
    NotOpen { id: u64 },

    #[error("proposal is expired ({id})")]
    Expired { id: u64 },

    #[error("not registered to vote (no voting power) at time of proposal creation")]
    NotRegistered {},

    #[error("already voted. this proposal does not support revoting")]
    AlreadyVoted {},

    #[error("already cast a vote with that option. change your vote to revote")]
    AlreadyCast {},

    #[error("proposal is not in 'passed' state")]
    NotPassed {},

    #[error("proposal has already been executed")]
    AlreadyExecuted {},

    #[error("proposal is closed")]
    Closed {},

    #[error("only rejected proposals may be closed")]
    WrongCloseStatus {},

    #[error("the DAO is currently inactive, you cannot create proposals")]
    InactiveDao {},

    #[error("min_voting_period and max_voting_period must have the same units (height or time)")]
    DurationUnitsConflict {},

    #[error("min voting period must be less than or equal to max voting period")]
    InvalidMinVotingPeriod {},

    #[error(
        "pre-propose modules must specify a proposer. lacking one, no proposer should be specified"
    )]
    InvalidProposer {},

    #[error(transparent)]
    Tag(#[from] TagError),

    #[error(
        "all proposals with deposits must be completed out (closed or executed) before migration"
    )]
    PendingProposals {},

    #[error("received a failed proposal hook reply with an invalid hook index: ({idx})")]
    InvalidHookIndex { idx: u64 },
}
