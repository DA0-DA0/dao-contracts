use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Uint128};
use cw20::Denom;
use cw_storage_plus::{Item, Map};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub start_block: u64,
    pub end_block: u64,
    pub payment_per_block: Uint128,
    pub total_amount: Uint128,
    pub denom: Denom,
    pub staking_contract: Addr,
    pub funded: bool,
    pub payment_block_delta: u64,
}

pub const CONFIG: Item<Config> = Item::new("config");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct LastClaim {
    pub block_height: u64,
    pub time: cosmwasm_std::Timestamp,
}
pub const LAST_CLAIMED: Map<Addr, LastClaim> = Map::new("last_claimed");
