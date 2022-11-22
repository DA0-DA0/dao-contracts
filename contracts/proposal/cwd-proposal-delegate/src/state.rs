use cosmwasm_std::{Addr, CosmosMsg, Empty};
use cw_storage_plus::{Map, Item};
use cw_utils::Expiration;

struct Config {
    admin: Addr,
}

struct Delegation {
    delegate: Addr,
    msgs: Vec<CosmosMsg<Empty>>,
    expiration: Option<Expiration>, 
    
    policy_revocable: bool,
    policy_allow_retry_on_failure: bool 
}

const DELEGATIONS: Map<u64, Delegation> = Map::new("delegations");

const CONFIG: Item<Config> = Item::new("config");
