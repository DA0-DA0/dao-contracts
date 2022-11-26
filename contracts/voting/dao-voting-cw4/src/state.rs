use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, SnapshotItem, SnapshotMap, Strategy};

pub const USER_WEIGHTS: SnapshotMap<&Addr, Uint128> = SnapshotMap::new(
    "user_weights",
    "user_weights__checkpoints",
    "user_weights__changelog",
    Strategy::EveryBlock,
);

pub const TOTAL_WEIGHT: SnapshotItem<Uint128> = SnapshotItem::new(
    "total_weight",
    "total_weight__checkpoints",
    "total_weight__changelog",
    Strategy::EveryBlock,
);

pub const GROUP_CONTRACT: Item<Addr> = Item::new("group_contract");
pub const DAO: Item<Addr> = Item::new("dao_address");
