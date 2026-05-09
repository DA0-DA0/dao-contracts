use cosmwasm_std::{CheckedMultiplyFractionError, OverflowError, StdError, Uint128};
use cw_utils::{ParseReplyError, PaymentError};
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Overflow(#[from] OverflowError),

    #[error(transparent)]
    Payment(#[from] PaymentError),

    #[error(transparent)]
    ParseReplyError(#[from] ParseReplyError),

    #[error("{0}")]
    Ownership(#[from] cw_ownable::OwnershipError),

    #[error("{0}")]
    CheckedMultiplyFraction(#[from] CheckedMultiplyFractionError),

    #[error("Cannot mint more tokens than the maximum supply of {max}")]
    CannotExceedMaxSupply { max: Uint128 },

    #[error("The commons is closed to new contributions")]
    CommonsClosed {},

    #[error("The commons is locked against liquidations")]
    CommonsHatch {},

    #[error("Contribution must be less than or equal to {max} and greater than or equal to {min}")]
    ContributionLimit { min: Uint128, max: Uint128 },

    #[error("Hatch phase config error {0}")]
    HatchPhaseConfigError(String),

    #[error("Invalid exit fee, must be less than 100%.")]
    InvalidExitFee {},

    #[error("Invalid subdenom: {subdenom:?}")]
    InvalidSubdenom { subdenom: String },

    #[error("Invalid phase, expected {expected:?}, actual {actual:?}")]
    InvalidPhase { expected: String, actual: String },

    #[error("Invalid sell amount")]
    MismatchedSellAmount {},

    #[error("Open phase config error {0}")]
    OpenPhaseConfigError(String),

    #[error("Sender {sender:?} is not in the hatcher allowlist.")]
    SenderNotAllowlisted { sender: String },

    #[error("Supply token error {0}")]
    SupplyTokenError(String),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Got a submessage reply with unknown id: {id}")]
    UnknownReplyId { id: u64 },

    #[error("Contract is paused")]
    Paused {},
}
