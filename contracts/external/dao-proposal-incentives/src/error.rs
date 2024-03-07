use cosmwasm_std::StdError;
use cw_denom::DenomError;
use cw_ownable::OwnershipError;
use cw_utils::{ParseReplyError, PaymentError};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    PaymentError(#[from] PaymentError),

    #[error("{0}")]
    DenomError(#[from] DenomError),

    #[error("{0}")]
    ParseReplyError(#[from] ParseReplyError),

    #[error("{0}")]
    OwnershipError(#[from] OwnershipError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("An unknown reply ID was received.")]
    UnknownReplyID {},

    #[error("No reward per proposal given")]
    NoRewardPerProposal {},

    #[error("Proposal module is inactive")]
    ProposalModuleIsInactive {},
}
