use cosmwasm_schema::cw_serde;
use cw_storage_plus::Item;

/// The governance module's configuration.
#[cw_serde]
pub struct Config {}

/// The current top level config for the module.  The "config" key was
/// previously used to store configs for v1 DAOs.
pub const CONFIG: Item<Config> = Item::new("config_v2");

pub struct CodeIdPair {
    pub v1_code_id: u64,
    pub v2_code_id: u64,
    pub migrate_msg: dao_proposal_single::msg::MigrateMsg,
}

impl CodeIdPair {
    pub const fn new(
        v1_code_id: u64,
        v2_code_id: u64,
        migrate_msg: dao_proposal_single::msg::MigrateMsg,
    ) -> CodeIdPair {
        CodeIdPair {
            v1_code_id,
            v2_code_id,
            migrate_msg,
        }
    }
}
