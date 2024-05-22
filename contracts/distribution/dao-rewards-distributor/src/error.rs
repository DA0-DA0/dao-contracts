use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error(transparent)]
    Ownable(#[from] cw_ownable::OwnershipError),

    #[error(transparent)]
    Cw20Error(#[from] cw20_base::ContractError),

    #[error("Invalid Cw20")]
    InvalidCw20 {},

    #[error("Invalid funds")]
    InvalidFunds {},

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

    #[error("Rewards distributor shutdown error: {0}")]
    ShutdownError(String),

    #[error("Denom already registered")]
    DenomAlreadyRegistered {},
}
