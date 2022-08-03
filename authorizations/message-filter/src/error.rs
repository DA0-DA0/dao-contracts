use cosmwasm_std::StdError;
use cw_auth_middleware::ContractError as AuthorizationError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Unauthorized(#[from] AuthorizationError),

    #[error("Custom Error val: {val:?}")]
    CustomError { val: String },

    #[error("Authorization not found")]
    NotFound {},
}
