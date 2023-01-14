use cw_storage_plus::Item;

use crate::types::{MigrationParams, TestState, ModulesAddrs};

/// Data we except from users in order to do the migrations correctly.
pub const MIGRATION_PARAMS: Item<MigrationParams> = Item::new("migration_params");
/// Holds data about the DAO before migration (so we can test against it after migration)
pub const TEST_STATE: Item<TestState> = Item::new("test_state");
/// Holds addresses for what we need to query for
pub const MODULES_ADDRS: Item<ModulesAddrs> = Item::new("test_state");