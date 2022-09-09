use cosmwasm_std::StdError;
use cw_denom::DenomError;
use thiserror::Error;

use voting::deposit::DepositError;

#[derive(Error, Debug, PartialEq)]
pub enum PreProposeError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error(transparent)]
    Denom(#[from] DenomError),

    #[error(transparent)]
    Deposit(#[from] DepositError),

    #[error("message sender is not proposal module")]
    NotModule {},

    #[error("message sender is not dao")]
    NotDao {},

    #[error("you must be a member of this DAO (have voting power) to create a proposal")]
    NotMember {},

    #[error("no denomination for withdrawal. specify a denomination to withdraw")]
    NoWithdrawalDenom {},

    #[error("nothing to withdraw")]
    NothingToWithdraw {},
}
