use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

use crate::contract::ProposeMessageInternal;

pub const APPROVER: Item<Addr> = Item::new("approver");
pub const PROPOSALS: Map<u64, ProposeMessageInternal> = Map::new("pending_proposals");
