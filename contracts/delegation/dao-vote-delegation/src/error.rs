use cosmwasm_std::{DivideByZeroError, OverflowError, StdError};
use cw_utils::PaymentError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error(transparent)]
    Overflow(#[from] OverflowError),

    #[error(transparent)]
    DivideByZero(#[from] DivideByZeroError),

    #[error(transparent)]
    Payment(#[from] PaymentError),

    #[error("semver parsing error: {0}")]
    SemVer(String),

    #[error("unauthorized")]
    Unauthorized {},

    #[error("delegate already registered")]
    DelegateAlreadyRegistered {},

    #[error("delegate not registered")]
    DelegateNotRegistered {},

    #[error("delegates cannot delegate to others")]
    DelegatesCannotDelegate {},

    #[error("undelegate before registering as a delegate")]
    UndelegateBeforeRegistering {},

    #[error("no voting power to delegate")]
    NoVotingPower {},

    #[error("cannot delegate to self")]
    CannotDelegateToSelf {},

    #[error("delegation already exists")]
    DelegationAlreadyExists {},

    #[error("delegation does not exist")]
    DelegationDoesNotExist {},

    #[error("cannot delegate more than 100% (current: {current}%)")]
    CannotDelegateMoreThan100Percent { current: String },

    #[error("invalid voting power percent")]
    InvalidVotingPowerPercent {},

    #[error("migration error: incorrect contract: expected {expected}, actual {actual}")]
    MigrationErrorIncorrectContract { expected: String, actual: String },

    #[error("migration error: invalid version: new {new}, current {current}")]
    MigrationErrorInvalidVersion { new: String, current: String },
}

impl From<semver::Error> for ContractError {
    fn from(err: semver::Error) -> Self {
        Self::SemVer(err.to_string())
    }
}
