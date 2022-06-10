use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Invalid Cw20")]
    InvalidCw20 {},

    #[error("Invalid Staking Contract")]
    InvalidStakingContract {},

    #[error("Zero eligible rewards")]
    ZeroRewards {},

    #[error("Rewards have already been distributed for this block")]
    RewardsDistributedForBlock {},
}
