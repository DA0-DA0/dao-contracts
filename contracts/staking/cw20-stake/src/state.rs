use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128};
use cw_controllers::Claims;
use cw_controllers::Hooks;
use cw_storage_plus::{Item, SnapshotItem, SnapshotMap, Strategy};
use cw_utils::Duration;

#[cw_serde]
pub struct Config {
    pub owner: Option<Addr>,
    pub manager: Option<Addr>,
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

/// The maximum number of claims that may be outstanding.
pub const MAX_CLAIMS: u64 = 100;

pub const CLAIMS: Claims = Claims::new("claims");

pub const BALANCE: Item<Uint128> = Item::new("balance");

// Hooks to contracts that will receive staking and unstaking messages
pub const HOOKS: Hooks = Hooks::new("hooks");
