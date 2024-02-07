use cosmwasm_std::{CheckedMultiplyFractionError, OverflowError, StdError};
use cw_denom::{CheckedDenom, DenomError};
use cw_ownable::OwnershipError;
use cw_utils::Expiration;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    DenomError(#[from] DenomError),

    #[error("{0}")]
    OwnershipError(#[from] OwnershipError),

    #[error("{0}")]
    CheckedMultiplyFractionError(#[from] CheckedMultiplyFractionError),

    #[error("{0}")]
    OverflowError(#[from] OverflowError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("NotExpired")]
    NotExpired { expiration: Expiration },

    #[error("AlreadyExpired")]
    AlreadyExpired {},

    #[error("Proposal module is inactive")]
    ProposalModuleIsInactive {},

    #[error("UnexpectedFunds")]
    UnexpectedFunds {
        expected: CheckedDenom,
        received: CheckedDenom,
    },
}
