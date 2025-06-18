use cosmwasm_std::StdError;
use cw_utils::PaymentError;
use thiserror::Error;

pub use cw_ownable::OwnershipError;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error(transparent)]
    Payment(#[from] PaymentError),

    #[error(transparent)]
    Ownership(#[from] OwnershipError),

    #[error("Unauthorized")]
    Unauthorized {},
}
