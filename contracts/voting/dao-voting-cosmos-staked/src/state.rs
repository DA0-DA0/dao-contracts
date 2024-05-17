use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, SnapshotItem, Strategy};

/// The address of the DAO this voting contract is connected to.
pub const DAO: Item<Addr> = Item::new("dao");

/// Keeps track of staked total over time.
pub const STAKED_TOTAL: SnapshotItem<Uint128> = SnapshotItem::new(
    "total_staked",
    "total_staked__checkpoints",
    "total_staked__changelog",
    Strategy::EveryBlock,
);
