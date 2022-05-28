use cosmwasm_std::Addr;
use cw_storage_plus::Map;

pub const AUTHORIZED: Map<Addr, cosmwasm_std::Empty> = Map::new("authorized");
