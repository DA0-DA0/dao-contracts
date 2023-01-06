use cosmwasm_std::{
    CheckedFromRatioError, DecimalRangeExceeded, DivideByZeroError, OverflowError, StdError,
    Uint128,
};
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

    #[error("{0}")]
    OverflowErr(#[from] OverflowError),

    #[error("Amount sent does not match vesting amount")]
    AmountDoesNotMatch,

    #[error("Cw20 contract does not match vesting denom")]
    Cw20DoesNotMatch,

    #[error("Fully vested")]
    FullyVested,

    #[error("No tokens have vested for this vesting payment")]
    NoFundsToClaim { claimed: Uint128 },

    #[error("Tokens for this vesting payment are not stakeable")]
    NotStakeable,

    #[error("Vesting Payment does not exist")]
    VestingPaymentNotFound { vesting_payment_id: u64 },

    #[error("The transfer will never become fully vested. Must hit 0 eventually")]
    NeverFullyVested,

    #[error("This account is unauthorized to perform the transaction")]
    Unauthorized,

    #[error("The transfer tries to vest more tokens than it sends")]
    VestsMoreThanSent,

    #[error("Incorrect coin denom")]
    IncorrectDenom {},

    #[error("Cannot undelegate more than you previously delegated")]
    InsufficientDelegation {},

    #[error("Cannot undelegate or claim rewards from a validator that does not have delegations")]
    NoDelegationsForValidator {},

    #[error("Contract has run out of funds to delegate")]
    NoFundsToDelegate {},

    #[error("Vesting payment does not have enough funds")]
    NotEnoughFunds {},

    #[error("Rewards amount is 0")]
    ZeroRewardsToSend {},

    #[error(transparent)]
    DivideByZeroError(#[from] DivideByZeroError),

    #[error(transparent)]
    DecimalRangeExceeded(#[from] DecimalRangeExceeded),

    #[error(transparent)]
    CheckedFromRatioError(#[from] CheckedFromRatioError),
}
