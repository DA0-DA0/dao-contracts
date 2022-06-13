use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum Kind {
    Allow {},
    Reject {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Authorization {
    pub kind: Kind,
    pub addr: Addr,
    pub matcher: String,
}

// TODO: Add config for the defaults
pub const DAO: Item<Addr> = Item::new("dao");
// TODO: Store map based on partial indices?
pub const ALLOWED: Map<Addr, Vec<Authorization>> = Map::new("allowed");
