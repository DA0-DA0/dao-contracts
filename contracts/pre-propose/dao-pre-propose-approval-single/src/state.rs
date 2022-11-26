use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, StdResult, Storage};
use cw_storage_plus::{Item, Map};

use dao_voting::deposit::CheckedDepositInfo;
use dao_voting::proposal::SingleChoiceProposeMsg as ProposeMsg;

#[cw_serde]
pub struct PendingProposal {
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
pub const PENDING_PROPOSALS: Map<u64, PendingProposal> = Map::new("pending_proposals");

/// Used internally to track the current approval_id.
const CURRENT_ID: Item<u64> = Item::new("current_id");

pub(crate) fn advance_approval_id(store: &mut dyn Storage) -> StdResult<u64> {
    let id: u64 = CURRENT_ID.may_load(store)?.unwrap_or_default() + 1;
    CURRENT_ID.save(store, &id)?;
    Ok(id)
}
