use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;
use cw_storage_plus::{SnapshotItem, SnapshotMap, Strategy};

/// A historic snapshot of total weight over time
pub const TOTAL: SnapshotItem<u64> = SnapshotItem::new(
    "total",
    "total__checkpoints",
    "total__changelog",
    Strategy::EveryBlock,
);

/// A historic list of members and total voting weights
pub const MEMBERS: SnapshotMap<&Addr, u64> = SnapshotMap::new(
    "members",
    "members__checkpoints",
    "members__changelog",
    Strategy::EveryBlock,
);
