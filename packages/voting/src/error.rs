use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum VotingError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("min_voting_period and max_voting_period must have the same units (height or time)")]
    DurationUnitsConflict {},

    #[error("Min voting period must be less than or equal to max voting period")]
    InvalidMinVotingPeriod {},
}
