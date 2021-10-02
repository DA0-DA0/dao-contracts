use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map, SnapshotMap, Strategy};

pub const BALANCES: SnapshotMap<&Addr, Uint128> = SnapshotMap::new(
    "balances",
    "balances__checkpoints",
    "balances__changelog",
    Strategy::EveryBlock,
);