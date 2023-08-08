use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128};
use cw_controllers::Claims;
use cw_storage_plus::{Item, SnapshotItem, SnapshotMap, Strategy};
use cw_utils::Duration;
use dao_voting::threshold::ActiveThreshold;

#[cw_serde]
pub struct Config {
    pub owner: Option<Addr>,
    pub manager: Option<Addr>,
    pub unstaking_duration: Option<Duration>,
}

pub const CONFIG: Item<Config> = Item::new("config");
pub const DENOM: Item<String> = Item::new("denom");
pub const DAO: Item<Addr> = Item::new("dao");
pub const STAKED_BALANCES: SnapshotMap<&Addr, Uint128> = SnapshotMap::new(
    "staked_balances",
    "staked_balance__checkpoints",
    "staked_balance__changelog",
    Strategy::EveryBlock,
);

pub const STAKED_TOTAL: SnapshotItem<Uint128> = SnapshotItem::new(
    "total_staked",
    "total_staked__checkpoints",
    "total_staked__changelog",
    Strategy::EveryBlock,
);

/// The maximum number of claims that may be outstanding.
pub const MAX_CLAIMS: u64 = 100;

pub const CLAIMS: Claims = Claims::new("claims");

pub const ACTIVE_THRESHOLD: Item<ActiveThreshold> = Item::new("active_threshold");
