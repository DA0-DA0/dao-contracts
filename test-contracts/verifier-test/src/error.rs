use cosmwasm_std::StdError;
use cw_verifier_middleware::error::ContractError as VerifyError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    VerifyError(#[from] VerifyError),

    #[error("Unauthorized")]
    Unauthorized {},
}
