use std::collections::BinaryHeap;

use cosmwasm_schema::cw_serde;
use cw_controllers::Claims;
use schemars::{JsonSchema};
use serde::{Deserialize, Serialize};
use cosmwasm_std::{Addr, Uint128, Timestamp};
use cw_storage_plus::{Item, Map, SnapshotMap, Strategy, SnapshotItem};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner: Option<Addr>,
    pub manager: Option<Addr>,
    pub token_address: Addr,
    pub stake_address: Addr,
    pub vest_total: Uint128,
}

#[cw_serde]
#[derive(Ord, PartialOrd, Eq)]
pub struct Vest {
    // Ordering of fields here is important, we want points to be sorted by
    // their expiration ascending first and then amount ascending.  Schedules
    // should implement vesting cliffs by setting two consecutive points at the
    // same time, where the first point's amount defines the bottom of the
    // discontinuity and the second point's amount defines the top.
    pub expiration: Timestamp,
    pub amount: Uint128,
}

/// The maximum number of claims that may be outstanding.
pub const MAX_CLAIMS: u64 = 100;

pub const CONFIG: Item<Config> = Item::new("config");
pub const SCHEDULES: Map<Addr, BinaryHeap<Vest>> = Map::new("schedules");
pub const ACTIVATED: SnapshotItem<bool> = SnapshotItem::new(
    "activated",
    "claimed_total__checkpoints",
    "claimed_total__changelog",
    Strategy::EveryBlock,
);
pub const CLAIMED_TOTAL: SnapshotMap<Addr, Uint128> = SnapshotMap::new(
    "claimed_total",
    "claimed_total__checkpoints",
    "claimed_total__changelog",
    Strategy::EveryBlock,
);
pub const CLAIMS: Claims = Claims::new("claims");
