use cosmwasm_std::{OverflowError, StdError};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error(transparent)]
    OverflowError(#[from] OverflowError),

    #[error("Unauthorized")]
    Unauthorized {},
}
