use cosmwasm_std::StdError;
use cw_hooks::HookError;
use thiserror::Error;

use dao_voting::veto::VetoError;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Veto(#[from] VetoError),

    #[error("{0}")]
    Hook(#[from] HookError),

    #[error("Unauthorized — sender is not the configured WAVS operator")]
    Unauthorized {},

    #[error("DAO is inactive — voting module reports active=false")]
    InactiveDao {},

    #[error("Replay rejected — attestation with this eventId has already been processed")]
    Replay { event_id_hex: String },

    #[error("Invalid envelope payload — could not decode as ProposalPayload: {reason}")]
    InvalidPayload { reason: String },

    #[error("Service-manager validation rejected the envelope: {reason}")]
    SignatureInvalid { reason: String },

    #[error("Mandate filter rejected msg #{index}: {reason}")]
    MandateFilterFail { index: usize, reason: String },

    #[error("Mandate filter encountered a fatal error on msg #{index}: {reason}")]
    MandateFilterFatal { index: usize, reason: String },

    #[error("Proposal #{id} not found")]
    ProposalNotFound { id: u64 },

    #[error("Proposal #{id} is not in a state that allows {action}")]
    InvalidProposalState { id: u64, action: String },

    #[error("Proposal too large: {size} bytes exceeds max {max}")]
    ProposalTooLarge { size: u64, max: u64 },

    #[error("Timelock not yet expired for proposal #{id}")]
    TimelockNotExpired { id: u64 },

    #[error("Only the DAO core may call this admin action")]
    NotDaoCore {},
}
