use cosmwasm_std::{Addr, StdResult, Storage};
use cw_storage_plus::{Item, Map};

use crate::{config::Config, proposal::Proposal, tally::Tally};

pub const DAO: Item<Addr> = Item::new("dao");
pub const CONFIG: Item<Config> = Item::new("config");

pub const TALLYS: Map<u32, Tally> = Map::new("tallys");
pub const PROPOSALS: Map<u32, Proposal> = Map::new("proposals");

pub(crate) fn next_proposal_id(storage: &dyn Storage) -> StdResult<u32> {
    PROPOSALS
        .keys(storage, None, None, cosmwasm_std::Order::Descending)
        .next()
        .transpose()
        .map(|id| id.unwrap_or(0) + 1)
}
