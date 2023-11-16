use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, StdResult, Storage};
use cw_storage_plus::{Item, Map};

use dao_voting::deposit::CheckedDepositInfo;
use dao_voting::proposal::SingleChoiceProposeMsg as ProposeMsg;

#[cw_serde]
pub enum ProposalStatus {
    /// The proposal is pending approval.
    Pending {},
    /// The proposal has been approved.
    Approved {
        /// The created proposal ID.
        created_proposal_id: u64,
    },
    /// The proposal has been rejected.
    Rejected {},
}

#[cw_serde]
pub struct Proposal {
    /// The status of a completed proposal.
    pub status: ProposalStatus,
    /// The approval ID used to identify this pending proposal.
    pub approval_id: u64,
    /// The address that created the proposal.
    pub proposer: Addr,
    /// The propose message that ought to be executed on the proposal
    /// message if this proposal is approved.
    pub msg: ProposeMsg,
    /// Snapshot of the deposit info at the time of proposal
    /// submission.
    pub deposit: Option<CheckedDepositInfo>,
}

pub const APPROVER: Item<Addr> = Item::new("approver");
pub const PENDING_PROPOSALS: Map<u64, Proposal> = Map::new("pending_proposals");
pub const COMPLETED_PROPOSALS: Map<u64, Proposal> = Map::new("completed_proposals");
pub const CREATED_PROPOSAL_TO_COMPLETED_PROPOSAL: Map<u64, u64> =
    Map::new("created_to_completed_proposal");

/// Used internally to track the current approval_id.
const CURRENT_ID: Item<u64> = Item::new("current_id");

pub(crate) fn advance_approval_id(store: &mut dyn Storage) -> StdResult<u64> {
    let id: u64 = CURRENT_ID.may_load(store)?.unwrap_or_default() + 1;
    CURRENT_ID.save(store, &id)?;
    Ok(id)
}
