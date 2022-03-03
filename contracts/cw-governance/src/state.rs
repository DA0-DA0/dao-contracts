use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Empty};
use cw_storage_plus::{Item, Map};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub name: String,
    pub description: String,
    pub image_url: Option<String>,
}

pub const CONFIG: Item<Config> = Item::new("config");
pub const VOTING_MODULE: Item<Addr> = Item::new("voting_module");
pub const GOVERNANCE_MODULES: Map<Addr, Empty> = Map::new("governance_modules");

/// Stores the number of governance modules present in the governance
/// contract. This information is avaliable from the governance
/// modules map but finding it requires a full traversal of the
/// keys. This means that we can't us that value when adding and
/// removing modules as it could cause the contract to lock due to gas
/// issues if too many modules are present.
pub const GOVERNANCE_MODULE_COUNT: Item<u64> = Item::new("governance_module_count");
