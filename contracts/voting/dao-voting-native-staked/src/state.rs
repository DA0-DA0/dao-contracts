use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128};
use cw_controllers::Claims;
use cw_hooks::Hooks;
use cw_storage_plus::{Item, SnapshotItem, SnapshotMap, Strategy};
use cw_utils::Duration;
use dao_voting::threshold::ActiveThreshold;

#[cw_serde]
pub struct Config {
    pub denom: String,
    pub unstaking_duration: Option<Duration>,
}

/// The configuration of this voting contract
pub const CONFIG: Item<Config> = Item::new("config");

/// The address of the DAO that instantiated this contract
pub const DAO: Item<Addr> = Item::new("dao");

/// Keeps track of staked balances by address over time
pub const STAKED_BALANCES: SnapshotMap<&Addr, Uint128> = SnapshotMap::new(
    "staked_balances",
    "staked_balance__checkpoints",
    "staked_balance__changelog",
    Strategy::EveryBlock,
);

/// Keeps track of limits by address
pub const LIMITS: SnapshotMap<&Addr, Uint128> = SnapshotMap::new(
    "limits",
    "limits__checkpoints",
    "limits__changelog",
    Strategy::EveryBlock,
);

/// Keeps track of staked total over time
pub const STAKED_TOTAL: SnapshotItem<Uint128> = SnapshotItem::new(
    "total_staked",
    "total_staked__checkpoints",
    "total_staked__changelog",
    Strategy::EveryBlock,
);

/// The maximum number of claims that may be outstanding.
pub const MAX_CLAIMS: u64 = 100;

pub const CLAIMS: Claims = Claims::new("claims");

/// The minimum amount of staked tokens for the DAO to be active
pub const ACTIVE_THRESHOLD: Item<ActiveThreshold> = Item::new("active_threshold");

/// Hooks to contracts that will receive staking and unstaking messages
pub const HOOKS: Hooks = Hooks::new("hooks");
