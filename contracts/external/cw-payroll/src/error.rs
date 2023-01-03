use cosmwasm_std::{StdError, Uint128};
use cw_denom::DenomError;
use cw_ownable::OwnershipError;
use cw_utils::PaymentError;
use thiserror::Error;
use wynd_utils::CurveError;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Curve(#[from] CurveError),

    #[error(transparent)]
    Denom(#[from] DenomError),

    #[error(transparent)]
    Ownable(#[from] OwnershipError),

    #[error("{0}")]
    PaymentError(#[from] PaymentError),

    #[error("Not authorized to perform action")]
    Unauthorized {},

    #[error("Fully vested")]
    FullyVested {},

    #[error("The start time is invalid. Start time must be before the end time and after the current block time")]
    InvalidStartTime {},

    #[error("The end time is invalid. End time must be before current block time")]
    InvalidEndTime {},

    #[error("No tokens have vested for this stream")]
    NoFundsToClaim { claimed: Uint128 },

    #[error("Vesting Payment does not exist")]
    VestingPaymentNotFound { vesting_payment_id: u64 },

    #[error("VestingcPayment recipient cannot be the stream owner")]
    InvalidRecipient {},

    #[error("Can not pause paused stream")]
    AlreadyPaused {},

    #[error("Vesting Payment is not paused for resume")]
    NotPaused {},

    #[error("Could not create bank transfer message")]
    CouldNotCreateBankMessage {},

    #[error("The transfer will never become fully vested. Must hit 0 eventually")]
    NeverFullyVested,

    #[error("The transfer tries to vest more tokens than it sends")]
    VestsMoreThanSent,
}
