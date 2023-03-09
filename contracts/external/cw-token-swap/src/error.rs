use cosmwasm_std::{StdError, Uint128};
use cw_utils::PaymentError;
use thiserror::Error;

#[derive(Error, Debug)]
#[cfg_attr(test, derive(PartialEq))] // Only neeed while testing.
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error(transparent)]
    PaymentError(#[from] PaymentError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Counterparties must have different addresses")]
    NonDistinctCounterparties {},

    #[error("Can not provide funds more than once")]
    AlreadyProvided {},

    #[error("Escrow funds have already been sent")]
    Complete {},

    #[error("Must provide funds before withdrawing")]
    NoProvision {},

    #[error("Can not create an escrow for zero tokens")]
    ZeroTokens {},

    #[error("Provided funds do not match promised funds")]
    InvalidFunds {},

    #[error("You are trying to send more funds then you have")]
    WrongFundsCalculation {},

    #[error("Send message doesn't match the other party token type")]
    InvalidSendMsg {},

    #[error("Invalid amount. Expected ({expected}), got ({actual})")]
    InvalidAmount { expected: Uint128, actual: Uint128 },
}
