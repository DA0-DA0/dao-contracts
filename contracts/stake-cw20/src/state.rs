use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Uint128};
use cw_controllers::Claims;
use cw_storage_plus::{Item, SnapshotItem, SnapshotMap, Strategy};
use cw_utils::Duration;

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Config {
    pub token_address: Addr,
    pub unstaking_duration: Option<Duration>,
}

pub const CONFIG: Item<Config> = Item::new("config");

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

pub const CLAIMS: Claims = Claims::new("claims");

pub const BALANCE: Item<Uint128> = Item::new("balance");
