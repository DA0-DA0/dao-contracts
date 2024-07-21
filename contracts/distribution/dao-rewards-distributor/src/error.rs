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

    #[error("Invalid Cw20")]
    InvalidCw20 {},

    #[error("Invalid funds")]
    InvalidFunds {},

    #[error("You cannot send native funds when registering a CW20")]
    NoFundsOnCw20Register {},

    #[error("Staking change hook sender is not staking contract")]
    InvalidHookSender {},

    #[error("No rewards claimable")]
    NoRewardsClaimable {},

    #[error("Reward period not finished")]
    RewardPeriodNotFinished {},

    #[error("Reward rate less then one per block")]
    RewardRateLessThenOnePerBlock {},

    #[error("Reward duration can not be zero")]
    ZeroRewardDuration {},

    #[error("All rewards have already been distributed")]
    RewardsAlreadyDistributed {},

    #[error("Denom already registered")]
    DenomAlreadyRegistered {},

    #[error("Denom not registered")]
    DenomNotRegistered {},
}
