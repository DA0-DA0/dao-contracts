use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{SnapshotMap, Strategy, Map};
use std::ops::Add;

pub const VOTING_POWER: SnapshotMap<&Addr, Uint128> = SnapshotMap::new(
    "voting_power",
    "voting_power__checkpoints",
    "voting_power__changelog",
    Strategy::EveryBlock,
);

// TODO: implement this feature
pub const DELEGATIONS: Map<&Addr,&Add> = Map::new("delegations")