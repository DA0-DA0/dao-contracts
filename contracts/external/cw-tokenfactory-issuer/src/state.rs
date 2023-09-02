use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map};

pub const OWNER: Item<Addr> = Item::new("owner");
pub const DENOM: Item<String> = Item::new("denom");

/// Blacklisted addresses prevented from transferring tokens
pub const BLACKLISTED_ADDRESSES: Map<&Addr, bool> = Map::new("blacklisted_addresses");

/// Addresses allowed to transfer tokens even if the token is frozen
pub const WHITELISTED_ADDRESSES: Map<&Addr, bool> = Map::new("whitelisted_addresses");

/// Whether or not features that require MsgBeforeSendHook are enabled
/// Many Token Factory chains do not yet support MsgBeforeSendHook
pub const BEFORE_SEND_HOOK_FEATURES_ENABLED: Item<bool> = Item::new("hook_features_enabled");

/// Whether or not token transfers are frozen
pub const IS_FROZEN: Item<bool> = Item::new("is_frozen");

// Address able to manange blacklists and whitelists
pub const BLACKLISTERS: Map<&Addr, bool> = Map::new("blacklisters");
pub const WHITELISTERS: Map<&Addr, bool> = Map::new("whitelisters");

/// Allowances
pub const BURNER_ALLOWANCES: Map<&Addr, Uint128> = Map::new("burner_allowances");
pub const FREEZER_ALLOWANCES: Map<&Addr, bool> = Map::new("freezer_allowances");
pub const MINTER_ALLOWANCES: Map<&Addr, Uint128> = Map::new("minter_allowances");
