use cosmwasm_std::{StdError, Uint128};
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),
    #[error("DivideByZero")]
    DivideByZero {},
    #[error("Unauthorized")]
    Unauthorized {},
    #[error("NotFunded")]
    NotFunded {},
    #[error("AlreadyFunded")]
    AlreadyFunded {},
    #[error("IncorrectDenom")]
    IncorrectDenom {},
    #[error("IncorrectFundingAmount")]
    IncorrectFundingAmount {
        received: Uint128,
        expected: Uint128,
    },
    #[error("RewardsNotStarted")]
    RewardsNotStarted {
        current_block: u64,
        start_block: u64,
    },
    #[error("ZeroClaimable")]
    ZeroClaimable {},
    // Add any other custom errors you like here.
    // Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.
    #[error("BlockInFuture")]
    InvalidFutureBlock {},
    #[error("Start block already occurred")]
    StartBlockAlreadyOccurred {},
    #[error("Start block later then end block")]
    StartBlockAfterEndBlock {},
    #[error("Start block is not divisible by blocks between payments")]
    StartAndEndBlocksNotDivisibleByBlocksBetweenPayments {},
    #[error("Total amount is not equal to total payments")]
    InvalidTotalAmount {},
}
