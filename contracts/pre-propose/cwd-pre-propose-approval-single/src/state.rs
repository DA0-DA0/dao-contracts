use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, CosmosMsg, Empty};
use cw_storage_plus::{Item, Map};

#[cw_serde]
pub struct ProposeMsgSingle {
    pub title: String,
    pub description: String,
    pub msgs: Vec<CosmosMsg<Empty>>,
    pub proposer: Option<String>,
}

#[cw_serde]
pub struct PendingProposal {
    pub id: u64,
    pub msg: ProposeMsgSingle,
}

pub const APPROVER: Item<Addr> = Item::new("approver");
pub const CURRENT_ID: Item<u64> = Item::new("current_id");
pub const PENDING_PROPOSALS: Map<u64, PendingProposal> = Map::new("pending_proposals");
