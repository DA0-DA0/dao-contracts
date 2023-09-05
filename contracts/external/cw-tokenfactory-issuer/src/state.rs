use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map};

/// Holds the owner of this contract
pub const OWNER: Item<Addr> = Item::new("owner");

/// Holds the Token Factory denom managed by this contract
pub const DENOM: Item<String> = Item::new("denom");

/// Denylist addresses prevented from transferring tokens
pub const DENYLIST: Map<&Addr, bool> = Map::new("denylist");

/// Addresses allowed to transfer tokens even if the token is frozen
pub const ALLOWLIST: Map<&Addr, bool> = Map::new("allowlist");

/// Whether or not features that require MsgBeforeSendHook are enabled
/// Many Token Factory chains do not yet support MsgBeforeSendHook
pub const BEFORE_SEND_HOOK_FEATURES_ENABLED: Item<bool> = Item::new("hook_features_enabled");

/// Whether or not token transfers are frozen
pub const IS_FROZEN: Item<bool> = Item::new("is_frozen");

/// Allowances for burning
pub const BURNER_ALLOWANCES: Map<&Addr, Uint128> = Map::new("burner_allowances");

/// Allowances for minting
pub const MINTER_ALLOWANCES: Map<&Addr, Uint128> = Map::new("minter_allowances");
