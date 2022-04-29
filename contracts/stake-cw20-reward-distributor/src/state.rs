use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner: Addr,
    pub staking_addr: Addr,
    pub reward_rate: Uint128,
    pub reward_token: Addr,
}

pub const CONFIG: Item<Config> = Item::new("config");

pub const LAST_PAYMENT_BLOCK: Item<u64> = Item::new("last_payment_block");
