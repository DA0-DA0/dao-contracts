use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Invalid distribution height. Height must be <= current block height.")]
    DistributionHeight {},

    #[error("Zero funds provided.")]
    ZeroFunds {},
}
