use cosmwasm_std::StdError;
use thiserror::Error;

use crate::vote::VoteError;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error(transparent)]
    Std(#[from] StdError),
    #[error(transparent)]
    InvalidVote(#[from] VoteError),
}
