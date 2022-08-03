use cosmwasm_std::{Addr, StdError};
use cw_utils::ParseReplyError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized.")]
    Unauthorized {},

    #[error("The contract is paused.")]
    Paused {},

    #[error("No voting module provided.")]
    NoVotingModule {},

    #[error("Execution would result in no governance modules being active.")]
    NoProposalModule {},

    #[error("An unknown reply ID was received.")]
    UnknownReplyID {},

    #[error("{0}")]
    ParseReplyError(#[from] ParseReplyError),

    #[error("Multiple voting modules during instantiation.")]
    MultipleVotingModules {},

    #[error("Unsigned integer overflow.")]
    Overflow {},

    #[error("Key is missing from storage")]
    KeyMissing {},

    #[error("No pending admin nomination.")]
    NoAdminNomination {},

    #[error(
        "The pending admin nomination must be withdrawn before a new nomination can be created."
    )]
    PendingNomination {},

    #[error("Proposal module with address ({address}) does not exist.")]
    ProposalModuleDoesNotExist { address: Addr },
}
