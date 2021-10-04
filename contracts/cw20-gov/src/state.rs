use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{SnapshotMap, Strategy};

pub const BALANCES: SnapshotMap<&Addr, Uint128> = SnapshotMap::new(
    "balances",
    "balances__checkpoints",
    "balances__changelog",
    Strategy::EveryBlock,
);
