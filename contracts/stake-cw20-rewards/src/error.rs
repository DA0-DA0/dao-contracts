use cosmwasm_std::{Addr, StdError};
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),
    #[error("Config invalid")]
    ConfigInvalid {},
    #[error("Contract already funded")]
    AlreadyFunded {},
    #[error("Incorrect funding amount")]
    IncorrectFundingAmount {},
    #[error("Invalid token")]
    InvalidToken { received: Addr, expected: Addr },
    #[error("Rewards not funded")]
    RewardsNotFunded {},
    #[error("Rewards not started")]
    RewardsNotStarted {},
    #[error("Rewards finished")]
    RewardsFinished {},
    #[error("Rewards already claimed")]
    RewardsAlreadyClaimed {},
}
