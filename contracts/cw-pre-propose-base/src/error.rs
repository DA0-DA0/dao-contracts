use cosmwasm_std::StdError;
use thiserror::Error;

use voting::deposit::DepositError;

#[derive(Error, Debug, PartialEq)]
pub enum PreProposeError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error(transparent)]
    Deposit(#[from] DepositError),

    #[error("message sender is not proposal module")]
    NotModule {},

    #[error("you must be a member of this DAO (have voting power) to create a proposal")]
    NotMember {},
}
