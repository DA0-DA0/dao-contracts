use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Required threshold cannot be zero")]
    ZeroThreshold {},

    #[error("Not possible to reach required (passing) threshold")]
    UnreachableThreshold {},

    #[error("Suggested proposal expiration is larger than the maximum proposal duration")]
    InvalidExpiration {},
}
