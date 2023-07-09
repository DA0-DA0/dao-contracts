use cosmwasm_std::StdError;
use cw_denom::DenomError;
use cw_ownable::OwnershipError;
use cw_utils::PaymentError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error(transparent)]
    Denom(#[from] DenomError),

    #[error(transparent)]
    Ownable(#[from] OwnershipError),

    #[error("{0}")]
    PaymentError(#[from] PaymentError),

    #[error("Cannot renounce ownership")]
    CannotRenounceOwnership,

    #[error("Wrong denom")]
    WrongDenom,

    #[error("Wrong amount")]
    WrongAmount,

    #[error("Name already registered")]
    NameAlreadyRegistered,

    #[error("Already registered")]
    AlreadyRegistered,

    #[error("Registration pending")]
    RegistrationPending,

    #[error("Name must be between 3 and 32 characters")]
    InvalidName,

    #[error("No registration found")]
    NoRegistrationFound,

    #[error("Registration already renewed")]
    RegistrationAlreadyRenewed,

    #[error("No pending registration found")]
    NoPendingRegistrationFound,
}
