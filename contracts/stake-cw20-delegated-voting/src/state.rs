use cosmwasm_std::{Addr, Uint128};

use cw_storage_plus::{Item, Map, SnapshotMap, Strategy};

pub const STAKING_CONTRACT: Item<Addr> = Item::new("staking_contract");

pub const VOTING_POWER: SnapshotMap<&Addr, Uint128> = SnapshotMap::new(
    "voting_power",
    "voting_power__checkpoints",
    "voting_power__changelog",
    Strategy::EveryBlock,
);

pub const DELEGATIONS: Map<Addr, Addr> = Map::new("delegations");

