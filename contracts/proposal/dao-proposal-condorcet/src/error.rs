use cosmwasm_std::StdError;
use dao_voting::{error::VotingError, reply::error::TagError, threshold::ThresholdError};
use thiserror::Error;

use crate::vote::VoteError;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error(transparent)]
    Std(#[from] StdError),
    #[error(transparent)]
    InvalidVote(#[from] VoteError),
    #[error(transparent)]
    Threshold(#[from] ThresholdError),
    #[error(transparent)]
    Voting(#[from] VotingError),
    #[error(transparent)]
    Tag(#[from] TagError),

    #[error("non-zero voting power required to perform this action")]
    ZeroVotingPower {},

    #[error("only proposals that are in the passed state may be executed")]
    Unexecutable {},

    #[error("only rejected proposals may be closed")]
    Unclosable {},

    #[error("only the DAO my perform this action")]
    NotDao {},

    #[error("already voted")]
    Voted {},

    #[error("only non-expired proposals may be voted on")]
    Expired {},

    #[error("must specify at least one choice for proposal")]
    ZeroChoices {},
}
