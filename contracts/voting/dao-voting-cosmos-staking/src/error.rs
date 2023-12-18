use cosmwasm_std::{OverflowError, StdError};
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error(transparent)]
    OverflowError(#[from] OverflowError),

    #[error("Execution is not enabled on this contract.")]
    NoExecution {},
}
