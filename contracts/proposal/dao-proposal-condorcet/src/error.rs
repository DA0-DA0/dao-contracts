use cosmwasm_std::StdError;
use dao_voting::{error::VotingError, threshold::ThresholdError};
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

    #[error("non-zero voting power required to perform this action")]
    ZeroVotingPower {},

    #[error("only proposals that are in the passed state may be executed")]
    Unexecutable {},

    #[error("only rejected proposals may be closed")]
    Unclosable {},
}
