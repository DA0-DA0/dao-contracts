use cosmwasm_std::{StdError, Uint128};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Can not provide funds more than once")]
    AlreadyProvided {},

    #[error("Escrow funds have already been sent")]
    Complete {},

    #[error("Must provide funds before withdrawing")]
    NoProvision {},

    #[error("Can not create an escrow for zero tokens")]
    ZeroTokens {},

    #[error("Provied funds do not match promised funds")]
    InvalidFunds {},

    #[error("Invalid amount. Expected ({expected}), got ({actual})")]
    InvalidAmount { expected: Uint128, actual: Uint128 },
}