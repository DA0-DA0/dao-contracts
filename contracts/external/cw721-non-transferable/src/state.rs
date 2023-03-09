use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;
use cw_storage_plus::Item;

use cw_storage_plus::{SnapshotItem, SnapshotMap, Strategy};

pub const TOTAL_KEY: &str = "total";
pub const TOTAL_KEY_CHECKPOINTS: &str = "total__checkpoints";
pub const TOTAL_KEY_CHANGELOG: &str = "total__changelog";

pub const MEMBERS_KEY: &str = "members";
pub const MEMBERS_CHECKPOINTS: &str = "members__checkpoints";
pub const MEMBERS_CHANGELOG: &str = "members__changelog";

/// A historic snapshot of total weight over time
pub const TOTAL: SnapshotItem<u64> = SnapshotItem::new(
    TOTAL_KEY,
    TOTAL_KEY_CHECKPOINTS,
    TOTAL_KEY_CHANGELOG,
    Strategy::EveryBlock,
);

/// A historic list of members and total voting weights
pub const MEMBERS: SnapshotMap<&Addr, u64> = SnapshotMap::new(
    MEMBERS_KEY,
    MEMBERS_CHECKPOINTS,
    MEMBERS_CHANGELOG,
    Strategy::EveryBlock,
);
