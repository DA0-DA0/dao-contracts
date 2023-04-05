use cosmwasm_std::{OverflowError, StdError};
use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum RolesContractError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error(transparent)]
    Base(#[from] cw721_base::ContractError),

    #[error(transparent)]
    HookError(#[from] cw_controllers::HookError),

    #[error("{0}")]
    OverflowErr(#[from] OverflowError),

    #[error(transparent)]
    Ownable(#[from] cw_ownable::OwnershipError),
}
