use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

// Stores the address of the pre-propose approval contract
pub const PRE_PROPOSE_APPROVAL_CONTRACT: Item<Addr> = Item::new("pre_propose_approval_contract");
// Maps proposal ids to pre-propose ids
pub const PROPOSAL_IDS: Map<u64, u64> = Map::new("proposal_ids");
