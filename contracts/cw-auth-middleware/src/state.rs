use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
pub struct Config {
    /// The address of the DAO that this authorization module is
    /// associated with.
    pub dao: Addr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Authorization {
    pub contract: Addr,
    // ...
}

pub const CONFIG: Item<Config> = Item::new("config");
pub const AUTHORIZATIONS: Map<&Addr, Vec<Authorization>> = Map::new("authorizations");
pub const GROUPS: Map<&str, Addr> = Map::new("groups");
