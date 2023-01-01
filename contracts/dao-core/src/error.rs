use cosmwasm_std::{Addr, StdError};
use cw_utils::ParseReplyError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error(transparent)]
    ParseReplyError(#[from] ParseReplyError),

    #[error("Unauthorized.")]
    Unauthorized {},

    #[error("The contract is paused.")]
    Paused {},

    #[error("No voting module provided.")]
    NoVotingModule {},

    #[error("Execution would result in no proposal modules being active.")]
    NoActiveProposalModules {},

    #[error("An unknown reply ID was received.")]
    UnknownReplyID {},

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

    #[error("Proposal module with address ({address}) is already disabled.")]
    ModuleAlreadyDisabled { address: Addr },

    #[error("Proposal module with address is disabled and cannot execute messages.")]
    ModuleDisabledCannotExecute { address: Addr },

    #[error("Duplicate initial item: ({item})")]
    DuplicateInitialItem { item: String },

    #[error("Can not migrate. Current version is up to date.")]
    AlreadyMigrated {},
}
