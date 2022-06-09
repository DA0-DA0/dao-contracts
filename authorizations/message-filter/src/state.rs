use cosmwasm_std::{Addr, CosmosMsg};
use cw_storage_plus::{Item, Map};

pub struct Authorization {
    value: String,
}

impl From<CosmosMsg> for Authorization {
    fn from(msg: CosmosMsg) -> Self {
        Authorization {
            value: "test".to_string(),
        }
    }
}

pub const DAO: Item<Addr> = Item::new("dao");
pub const ALLOWED: Map<Addr, Vec<Authorization>> = Map::new("allowed");
