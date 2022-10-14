use crate::msg::{ActiveThreshold, VestingInfo};
use cosmwasm_std::Addr;
use cw_storage_plus::Item;
use cw_utils::Duration;

pub const ACTIVE_THRESHOLD: Item<ActiveThreshold> = Item::new("active_threshold");
pub const TOKEN: Item<Addr> = Item::new("token");
pub const DAO: Item<Addr> = Item::new("dao");
pub const STAKING_CONTRACT: Item<Addr> = Item::new("staking_contract");
pub const STAKING_CONTRACT_UNSTAKING_DURATION: Item<Option<Duration>> =
    Item::new("staking_contract_unstaking_duration");
pub const STAKING_CONTRACT_CODE_ID: Item<u64> = Item::new("staking_contract_code_id");

pub const VESTING_CONTRACT: Item<Option<Addr>> = Item::new("vesting_contract");
pub const VESTING_INFO: Item<Option<VestingInfo>> = Item::new("vesting_info");
