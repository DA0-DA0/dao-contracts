use cosmwasm_std::{OverflowError, StdError};
use cw_utils::PaymentError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error(transparent)]
    Ownable(#[from] cw_ownable::OwnershipError),

    #[error(transparent)]
    Cw20Error(#[from] cw20_base::ContractError),

    #[error(transparent)]
    Overflow(#[from] OverflowError),

    #[error(transparent)]
    Payment(#[from] PaymentError),

    #[error("Invalid CW20")]
    InvalidCw20 {},

    #[error("Invalid funds")]
    InvalidFunds {},

    #[error("You cannot send native funds when creating a CW20 distribution")]
    NoFundsOnCw20Create {},

    #[error("Voting power changed hook sender incorrect")]
    InvalidHookSender {},

    #[error("No rewards claimable")]
    NoRewardsClaimable {},

    #[error("All rewards have already been distributed")]
    RewardsAlreadyDistributed {},

    #[error("Distribution not found with ID {id}")]
    DistributionNotFound { id: u64 },

    #[error("Unexpected duplicate distribution with ID {id}")]
    UnexpectedDuplicateDistributionId { id: u64 },

    #[error("Invalid emission rate: {field} cannot be zero")]
    InvalidEmissionRateFieldZero { field: String },
}
