use cosmwasm_std::Addr;
use cw_controllers::Hooks;
use cw_storage_plus::{SnapshotItem, SnapshotMap, Strategy};

// Hooks to contracts that will receive staking and unstaking messages.
pub const HOOKS: Hooks = Hooks::new("hooks");

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
