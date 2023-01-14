use cosmwasm_std::{
    CheckedFromRatioError, DecimalRangeExceeded, DivideByZeroError, OverflowError, StdError,
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

    #[error(transparent)]
    CheckedFromRatioError(#[from] CheckedFromRatioError),

    #[error("{0}")]
    Curve(#[from] CurveError),

    #[error(transparent)]
    Denom(#[from] DenomError),

    #[error(transparent)]
    DivideByZeroError(#[from] DivideByZeroError),

    #[error(transparent)]
    DecimalRangeExceeded(#[from] DecimalRangeExceeded),

    #[error(transparent)]
    Ownable(#[from] OwnershipError),

    #[error("{0}")]
    OverflowErr(#[from] OverflowError),

    #[error("{0}")]
    PaymentError(#[from] PaymentError),

    #[error("Contract has already been funded")]
    AlreadyFunded,

    #[error("Amount sent does not match vesting amount")]
    AmountDoesNotMatch,

    #[error("Cw20 contract does not match vesting denom")]
    Cw20DoesNotMatch,

    #[error("Fully vested")]
    FullyVested,

    #[error("Title must be less than 280 characters and not be an empty string")]
    InvalidTitle,

    #[error("Cannot undelegate more than you previously delegated")]
    InsufficientDelegation {},

    #[error("Only callable if vesting payment is active")]
    NotActive,

    #[error("Must redelegate to a different validator")]
    SameValidator,

    #[error("Vesting payment does not have enough funds")]
    NotEnoughFunds {},

    #[error("Cannot undelegate or claim rewards from a validator that does not have delegations")]
    NoDelegationsForValidator {},

    #[error("No tokens have vested at the moment")]
    NoFundsToClaim,

    #[error("Contract has run out of funds to delegate")]
    NoFundsToDelegate {},

    #[error("Only callable if contract has been canceled and is unbonding")]
    NotCanceledAndUnbonding,

    #[error("Tokens for this vesting payment are not stakeable")]
    NotStakeable,

    #[error("The transfer will never become fully vested. Must hit 0 eventually")]
    NeverFullyVested,

    #[error("This account is unauthorized to perform the transaction")]
    Unauthorized,

    #[error("The transfer tries to vest more tokens than it sends")]
    VestsMoreThanSent,

    #[error("Vesting Payment has been cancelled by contract owner")]
    VestingPaymentCanceled,

    #[error("Vesting Payment does not exist")]
    VestingPaymentNotFound,

    #[error("Rewards amount is 0")]
    ZeroRewardsToSend {},

    #[error("Validator is not delegated to")]
    ValidatorNotDelegatedTo {},
}
