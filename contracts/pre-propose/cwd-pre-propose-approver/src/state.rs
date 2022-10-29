use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

// Stores the address of the pre-propose approval contract
pub const PRE_PROPOSE_APPROVAL_CONTRACT: Item<Addr> = Item::new("pre_propose_approval_contract");
// Stores the current pre-propose-id for use in submessage reply
pub const CURRENT_PRE_PROPOSE_ID: Item<u64> = Item::new("current_pre_propose_id");
// Maps proposal ids to pre-propose ids
pub const PROPOSAL_IDS: Map<u64, u64> = Map::new("proposal_ids");
