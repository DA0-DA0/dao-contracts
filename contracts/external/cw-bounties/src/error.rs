use cosmwasm_std::{StdError, Uint128};
use cw_ownable::OwnershipError;
use cw_utils::PaymentError;
use thiserror::Error;

#[derive(Error, Debug)]
#[cfg_attr(test, derive(PartialEq))] // Only neeed while testing.
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error(transparent)]
    Ownable(#[from] OwnershipError),

    #[error("{0}")]
    PaymentError(#[from] PaymentError),

    #[error("Title cannot be an empty string")]
    EmptyTitle {},

    #[error("Bounty is not open")]
    NotOpen {},

    #[error("Invalid amount. Expected ({expected}), got ({actual})")]
    InvalidAmount { expected: Uint128, actual: Uint128 },
}
