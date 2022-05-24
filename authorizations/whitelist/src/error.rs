use cosmwasm_std::StdError;
use thiserror::Error;
use cw_auth_manager::ContractError as AuthorizationError;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Unauthorized(#[from] AuthorizationError),

    #[error("Custom Error val: {val:?}")]
    CustomError { val: String },
}
