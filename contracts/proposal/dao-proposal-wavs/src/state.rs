use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, CosmosMsg, Empty};
use cw_hooks::Hooks;
use cw_storage_plus::{Item, Map};
use dao_voting::status::Status;
use dao_voting::veto::VetoConfig;

/// The proposal-module configuration. `_v2` suffix is reserved for migration headroom — v0.1
/// uses `_v1` so the next config rev can land cleanly.
pub const CONFIG: Item<Config> = Item::new("config_v1");

/// Total number of proposals ever submitted.
pub const PROPOSAL_COUNT: Item<u64> = Item::new("proposal_count");

/// Map<proposal_id, WavsProposal>.
pub const PROPOSALS: Map<u64, WavsProposal> = Map::new("proposals_v1");

/// Replay protection — keyed by `envelope.eventId` (bytes20). Once seen, never re-accepted.
pub const ATTESTATIONS_SEEN: Map<&[u8], bool> = Map::new("attestations_seen_v1");

/// Hooks that fire on proposal creation / state-change. Same pattern as dao-proposal-single.
pub const PROPOSAL_HOOKS: Hooks = Hooks::new("proposal_hooks");

/// The proposal-module configuration set at instantiate.
#[cw_serde]
pub struct Config {
    /// Address of the DAO core that owns this proposal module.
    pub dao: Addr,

    /// The WAVS service-manager contract this proposal module trusts for envelope validation.
    /// Service-handler defers `WavsValidate { envelope, signature_data }` to this address.
    pub service_manager: Addr,

    /// Authorized WAVS service identity. v1 supports a single operator address.
    pub authorized_service: AuthorizedService,

    /// Optional cw-filter mandate configuration. If present, every msg in the proposal payload
    /// is queried against `mandate_filter.filter_contract` before the proposal is queued.
    pub mandate_filter: Option<MandateFilterConfig>,

    /// Optional veto configuration. Same shape as dao-proposal-single. If set, proposals enter a
    /// timelock between accept and execute, during which `vetoer` can call Veto.
    pub veto: Option<VetoConfig>,

    /// If true, an accepted proposal auto-executes immediately (after timelock if veto is set).
    /// If false, an explicit Execute { proposal_id } call is required.
    pub auto_execute: bool,

    /// If true, a proposal whose execution fails is auto-closed. Mirrors dao-proposal-single.
    pub close_proposal_on_execution_failure: bool,
}

/// Authorized WAVS service. v1 supports a single-operator gate; quorum + registry come in v2.
#[cw_serde]
pub enum AuthorizedService {
    /// A single WAVS operator address is the only one allowed to submit attested envelopes.
    SingleOperator { addr: Addr },
    /// k-of-n operator quorum. (v2 — not handled in v0.1 verify path.)
    Quorum {
        operators: Vec<Addr>,
        threshold: u32,
    },
    /// Indirection through an external registry contract. (v2.)
    Registry { addr: Addr },
}

/// Configuration for the cw-filter mandate gate.
#[cw_serde]
pub struct MandateFilterConfig {
    /// The cw-filter contract to query.
    pub filter_contract: Addr,
    /// The JSON filter spec passed to `Filter { filter, msg }`.
    pub filter: serde_json::Value,
}

/// A WAVS-attested proposal record on this contract.
#[cw_serde]
pub struct WavsProposal {
    /// Human-readable title (from `ProposalPayload.title`).
    pub title: String,
    /// Human-readable description (from `ProposalPayload.description`).
    pub description: String,
    /// The hash of the original envelope.eventId — for cross-reference / audit.
    pub event_id_hex: String,
    /// The msgs to execute on the DAO core when the proposal finalizes.
    pub msgs: Vec<CosmosMsg<Empty>>,
    /// Block height at which the proposal was accepted.
    pub start_height: u64,
    /// Optional veto config snapshot — captured at create time.
    pub veto: Option<VetoConfig>,
    /// Whether auto-execute was requested.
    pub auto_execute: bool,
    /// Current status. Reuses dao_voting::status::Status; new variants added later if needed.
    pub status: Status,
}
