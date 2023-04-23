use cosmwasm_std::StdError;
use cw_utils::PaymentError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Payment(#[from] PaymentError),

    #[error("Invalid subdenom: {subdenom:?}")]
    InvalidSubdenom { subdenom: String },

    #[error("{0}")]
    Ownership(#[from] cw_ownable::OwnershipError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Hatch phase config error {0}")]
    HatchPhaseConfigError(String),

    #[error("Open phase config error {0}")]
    OpenPhaseConfigError(String),

    #[error("Supply token error {0}")]
    SupplyTokenError(String),

    #[error("Sender {sender:?} is not in the hatcher allowlist.")]
    SenderNotAllowlisted { sender: String },

    #[error("The commons is closed to new contributions")]
    CommonsClosed {},
}
