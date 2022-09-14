use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),
    #[error("{0}")]
    Cw20Error(#[from] cw20_base::ContractError),
    #[error("Unauthorized")]
    Unauthorized {},
    #[error("No rewards claimable")]
    NoRewardsClaimable {},
    #[error("Reward period already finished")]
    RewardPeriodFinished {},
    #[error("Reward period not finished")]
    RewardPeriodNotFinished {},
    #[error("Invalid funds")]
    InvalidFunds {},
    #[error("Invalid Cw20")]
    InvalidCw20 {},
    #[error("Reward rate less then one per block")]
    RewardRateLessThenOnePerBlock {},
    #[error("Reward duration can not be zero")]
    ZeroRewardDuration {},
    #[error("Contract is already in current state")]
    SettingCurrentPauseState {},
    #[error("Contract is paused")]
    ContractPaused {},
}
