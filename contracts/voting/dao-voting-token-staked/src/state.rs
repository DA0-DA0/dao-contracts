use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128};
use cw_controllers::Claims;
use cw_hooks::Hooks;
use cw_storage_plus::{Item, SnapshotItem, SnapshotMap, Strategy};
use cw_utils::Duration;
use dao_voting::threshold::ActiveThreshold;

use crate::msg::TokenInfo;

#[cw_serde]
pub struct Config {
    pub unstaking_duration: Option<Duration>,
}

/// The configuration of this voting contract
pub const CONFIG: Item<Config> = Item::new("config");

/// The address of the DAO this voting contract is connected to
pub const DAO: Item<Addr> = Item::new("dao");

/// The native denom associated with this contract
pub const DENOM: Item<String> = Item::new("denom");

/// Keeps track of staked balances by address over time
pub const STAKED_BALANCES: SnapshotMap<&Addr, Uint128> = SnapshotMap::new(
    "staked_balances",
    "staked_balance__checkpoints",
    "staked_balance__changelog",
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

/// Temporarily holds token_instantiation_info when creating a new Token Factory denom
pub const TOKEN_INSTANTIATION_INFO: Item<TokenInfo> = Item::new("token_instantiation_info");

/// The address of the cw-tokenfactory-issuer contract
pub const TOKEN_ISSUER_CONTRACT: Item<Addr> = Item::new("token_issuer_contract");
