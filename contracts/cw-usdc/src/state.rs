use std::ops::Add;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub denom: String,
    pub owner: Addr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct BeforeSendState {
    pub blacklisted_addresses: Map<'a, Addr, bool>,
    pub is_frozen: bool,
}

pub const CONFIG: Item<Config> = Item::new("config");
pub const BEFORESEND_STATE: Item<Config> = Item::new("beforesend_state");

pub const MINTER_ALLOWANCES: Map<Addr, Option<Uint128>> = Map::new("minter_allowances");
pub const BURNER_ALLOWANCES: Map<Addr, Option<Uint128>> = Map::new("burner_allowances");
pub const BLACKLISTER_ALLOWANCES: Map<Addr, bool> = Map::new("blacklister_allowances");
pub const FREEZER_ALLOWANCES: Map<Addr, bool> = Map::new("freezer_allowances");
