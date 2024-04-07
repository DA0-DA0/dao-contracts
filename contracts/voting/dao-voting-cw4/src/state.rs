use cosmwasm_std::Addr;
use cw_storage_plus::Item;
use dao_voting::threshold::ActiveThreshold;

pub const GROUP_CONTRACT: Item<Addr> = Item::new("group_contract");
pub const DAO: Item<Addr> = Item::new("dao_address");

/// The minimum amount of users for the DAO to be active
pub const ACTIVE_THRESHOLD: Item<ActiveThreshold> = Item::new("active_threshold");
