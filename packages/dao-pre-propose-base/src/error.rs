use cosmwasm_std::StdError;
use cw_denom::DenomError;
use cw_utils::ParseReplyError;
use thiserror::Error;

use cw_hooks::HookError;
use dao_voting::{
    deposit::DepositError, pre_propose::PreProposeSubmissionPolicyError, status::Status,
};

#[derive(Error, Debug, PartialEq)]
pub enum PreProposeError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error(transparent)]
    Denom(#[from] DenomError),

    #[error(transparent)]
    Deposit(#[from] DepositError),

    #[error(transparent)]
    Hooks(#[from] HookError),

    #[error(transparent)]
    ParseReplyError(#[from] ParseReplyError),

    #[error(transparent)]
    SubmissionPolicy(#[from] PreProposeSubmissionPolicyError),

    #[error("Message sender is not proposal module")]
    NotModule {},

    #[error("Message sender is not dao")]
    NotDao {},

    #[error("No denomination for withdrawal. specify a denomination to withdraw")]
    NoWithdrawalDenom {},

    #[error("Nothing to withdraw")]
    NothingToWithdraw {},

    #[error("Proposal status ({status}) is not completed")]
    NotCompleted { status: Status },

    #[error("Proposal not found")]
    ProposalNotFound {},

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("An unknown reply ID was received.")]
    UnknownReplyID {},

    #[error("Unsupported")]
    Unsupported {},
}
