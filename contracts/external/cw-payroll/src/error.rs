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

    #[error("Amount sent does not match vesting amount")]
    AmountDoesNotMatch,

    #[error("Cw20 contract does not match vesting denom")]
    Cw20DoesNotMatch,

    #[error("Fully vested")]
    FullyVested,

    #[error("No tokens have vested for this vesting payment")]
    NoFundsToClaim { claimed: Uint128 },

    #[error("Vesting Payment does not exist")]
    VestingPaymentNotFound { vesting_payment_id: u64 },

    #[error("The transfer will never become fully vested. Must hit 0 eventually")]
    NeverFullyVested,

    #[error("The transfer tries to vest more tokens than it sends")]
    VestsMoreThanSent,
}
