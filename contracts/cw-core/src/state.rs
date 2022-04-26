use cw_utils::Expiration;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Empty};
use cw_storage_plus::{Item, Map};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    /// The name of the contract.
    pub name: String,
    /// A description of the contract.
    pub description: String,
    /// An optional image URL for displaying alongside the contract.
    pub image_url: Option<String>,

    /// If true the contract will automatically add received cw20
    /// tokens to its treasury.
    pub automatically_add_cw20s: bool,
    /// If true the contract will automatically add received cw721
    /// tokens to its treasury.
    pub automatically_add_cw721s: bool,
}

pub const CONFIG: Item<Config> = Item::new("config");

pub const PAUSED: Item<Expiration> = Item::new("paused");

/// The voting module associated with this contract.
pub const VOTING_MODULE: Item<Addr> = Item::new("voting_module");
pub const PROPOSAL_MODULES: Map<Addr, Empty> = Map::new("governance_modules");

pub const ITEMS: Map<String, Addr> = Map::new("items");
pub const PENDING_ITEM_INSTANTIATION_NAMES: Map<u64, String> =
    Map::new("pending_item_instantiations");

/// Set of cw20 tokens that have been registered with this contract's
/// treasury.
pub const CW20_LIST: Map<Addr, Empty> = Map::new("cw20s");
/// Set of cw721 tokens that have been registered with this contract's
/// treasury.
pub const CW721_LIST: Map<Addr, Empty> = Map::new("cw721s");

/// Stores the number of governance modules present in the governance
/// contract. This information is avaliable from the governance
/// modules map but finding it requires a full traversal of the
/// keys. This means that we can't us that value when adding and
/// removing modules to check that at least one is present as it could
/// cause the contract to lock due to gas issues if too many modules
/// are present.
pub const PROPOSAL_MODULE_COUNT: Item<u64> = Item::new("governance_module_count");
