use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map, SnapshotMap, Strategy};
use cw_utils::Duration;

pub const TOKEN: Item<Addr> = Item::new("token");

pub const STAKING_CONTRACT: Item<Addr> = Item::new("staking_contract");
pub const STAKING_CONTRACT_UNSTAKING_DURATION: Item<Option<Duration>> =
    Item::new("staking_contract_unstaking_duration");
pub const STAKING_CONTRACT_CODE_ID: Item<u64> = Item::new("staking_contract_code_id");

pub const VOTE_WEIGHTS: SnapshotMap<&Addr, Uint128> = SnapshotMap::new(
    "user_weights",
    "user_weights__checkpoints",
    "user_weights__changelog",
    Strategy::EveryBlock,
);

pub const HEIGHT_TO_TOTAL_POWER: Map<u64, Uint128> = Map::new("height_to_total_power");

pub const DAO_ADDRESS: Item<Addr> = Item::new("dao_address");
