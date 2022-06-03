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

/// A contract
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Authorization {
    pub name: String,
    pub contract: Addr,
    //pub expiration: DateTime
    // ...
}

pub const CONFIG: Item<Config> = Item::new("config");
pub const AUTHORIZATIONS: Map<&Addr, Vec<Authorization>> = Map::new("authorizations");
pub const PROPOSAL_MODULE: Item<Addr> = Item::new("proposal_module");
