use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Empty, StdError, StdResult, Storage, Uint128};
use cw721_controllers::NftClaims;
use cw_controllers::Hooks;
use cw_storage_plus::{Item, Map, SnapshotItem, SnapshotMap, Strategy};
use cw_utils::Duration;

use crate::ContractError;

#[cw_serde]
pub struct Config {
    pub owner: Option<Addr>,
    pub nft_address: Addr,
}

pub const CONFIG: Item<Config> = Item::new("config");
pub const DAO: Item<Addr> = Item::new("dao");

// Hooks to contracts that will receive staking and unstaking
// messages.
pub const HOOKS: Hooks = Hooks::new("hooks");
