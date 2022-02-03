use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Uint128};
use cw_controllers::Claims;
use cw_storage_plus::{Item, SnapshotItem, SnapshotMap, Strategy};
use cw_utils::Duration;

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Config {
    pub token_address: Addr,
    pub staking_contract: Addr,
    pub payment_per_block: Uint128,
    pub total_payment: Uint128,
    pub start_block: u64,
    pub end_block: u64,
    pub funded: bool,
}

pub const CONFIG: Item<Config> = Item::new("config");

pub const LAST_CLAIM: Item<u64> = Iten::new("last_claim");
