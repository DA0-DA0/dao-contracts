use cosmwasm_schema::{cw_serde, QueryResponses};
use dao_voting::veto::VetoConfig;

use crate::state::{AuthorizedService, MandateFilterConfig};
pub use crate::state::WavsProposal;
use crate::wavs_compat::{ServiceHandlerExecuteMessages, ServiceHandlerQueryMessages};

/// Instantiate the dao-proposal-wavs contract. Called by the DAO core during `voting_module_instantiate_info`
/// or `proposal_modules_instantiate_info` setup.
#[cw_serde]
pub struct InstantiateMsg {
    /// The WAVS service-manager contract address to defer envelope validation to.
    pub service_manager: String,

    /// The authorized WAVS service identity. v1 supports `SingleOperator`.
    pub authorized_service: AuthorizedService,

    /// Optional cw-filter mandate configuration.
    pub mandate_filter: Option<MandateFilterConfig>,

    /// Optional veto + timelock configuration. Same shape as dao-proposal-single.
    pub veto: Option<VetoConfig>,

    /// Whether to auto-execute on attestation accept (subject to timelock if veto is configured).
    pub auto_execute: bool,

    /// Mirror of dao-proposal-single's same-named field.
    pub close_proposal_on_execution_failure: bool,
}

/// Execute messages for the contract. Embeds the canonical WAVS service-handler messages via
/// `#[serde(untagged)]` per the wavs-types convention so that WAVS calls into us with its
/// standard schema while we keep our DAO-specific messages in the same enum.
#[cw_serde]
pub enum ExecuteMsg {
    /// Execute a passed proposal. After timelock if veto is configured.
    Execute { proposal_id: u64 },

    /// Veto a proposal during its timelock window. Callable only by `veto.vetoer`.
    Veto { proposal_id: u64 },

    /// Close a proposal that has failed (either rejected by filter or expired).
    Close { proposal_id: u64 },

    /// Update authorized service. DAO-only.
    UpdateAuthorizedService { service: AuthorizedService },

    /// Update the mandate filter (or remove it). DAO-only.
    UpdateMandateFilter {
        mandate_filter: Option<MandateFilterConfig>,
    },

    /// Update veto config (or remove it). DAO-only.
    UpdateVeto { veto: Option<VetoConfig> },

    /// Update auto-execute toggle. DAO-only.
    UpdateAutoExecute { auto_execute: bool },

    /// Add a proposal hook receiver.
    AddProposalHook { address: String },

    /// Remove a proposal hook receiver.
    RemoveProposalHook { address: String },

    /// WAVS service-handler interface — receives signed envelopes from authorized operators.
    /// `#[serde(untagged)]` keeps the canonical wavs-types message shape.
    #[serde(untagged)]
    ServiceHandler(ServiceHandlerExecuteMessages),
}

/// Query messages. Mirrors dao-proposal-single's surface where applicable; embeds the canonical
/// service-handler queries via `#[serde(untagged)]`.
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Returns the proposal-module Config.
    #[returns(crate::state::Config)]
    Config {},

    /// Returns the total number of proposals ever submitted.
    #[returns(u64)]
    ProposalCount {},

    /// Returns a single proposal by id.
    #[returns(WavsProposal)]
    Proposal { proposal_id: u64 },

    /// Returns a paginated list of proposals.
    #[returns(Vec<WavsProposal>)]
    ListProposals {
        start_after: Option<u64>,
        limit: Option<u32>,
    },

    /// Returns whether a given event_id has already been seen (replay check).
    #[returns(bool)]
    EventIdSeen { event_id_hex: String },

    /// WAVS service-handler interface — exposes `WavsServiceManager {}` query.
    /// Returns `cosmwasm_std::Addr` (the configured service-manager address).
    #[returns(cosmwasm_std::Addr)]
    #[serde(untagged)]
    ServiceHandler(ServiceHandlerQueryMessages),
}

/// Migration message — empty for v0.1.
#[cw_serde]
pub struct MigrateMsg {}

/// The application-defined payload that lives inside `Envelope.payload` (ABI-encoded bytes
/// are decoded into this once the envelope is unwrapped).
#[cw_serde]
pub struct ProposalPayload {
    pub title: String,
    pub description: String,
    pub msgs: Vec<cosmwasm_std::CosmosMsg<cosmwasm_std::Empty>>,
}
