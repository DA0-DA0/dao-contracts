use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};
use cwd_proposal_single::msg::ProposeMsg;

#[cw_serde]
pub struct PendingProposal {
    pub id: u64,
    pub msg: ProposeMsg,
}

pub const APPROVER: Item<Addr> = Item::new("approver");
pub const CURRENT_ID: Item<u64> = Item::new("current_id");
pub const PENDING_PROPOSALS: Map<u64, PendingProposal> = Map::new("pending_proposals");
