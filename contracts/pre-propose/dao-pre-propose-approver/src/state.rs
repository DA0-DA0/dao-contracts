use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

// Stores the address of the pre-propose approval contract
pub const PRE_PROPOSE_APPROVAL_CONTRACT: Item<Addr> = Item::new("pre_propose_approval_contract");
// Maps proposal ids to pre-propose ids
pub const PROPOSAL_ID_TO_PRE_PROPOSE_ID: Map<u64, u64> = Map::new("proposal_to_pre_propose");
// Maps pre-propose ids to proposal ids
pub const PRE_PROPOSE_ID_TO_PROPOSAL_ID: Map<u64, u64> = Map::new("pre_propose_to_proposal");
