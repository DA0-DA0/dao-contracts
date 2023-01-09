use cosmwasm_schema::cw_serde;
use cw_storage_plus::Item;

/// The governance module's configuration.
#[cw_serde]
pub struct MigrationParams {
    //dao_proposal_single
    pub close_proposal_on_execution_failure: bool,
    pub pre_propose_info: dao_voting::pre_propose::PreProposeInfo,
    //dao_core
    pub dao_uri: Option<String>,
}

/// The current top level config for the module.  The "config" key was
/// previously used to store configs for v1 DAOs.
pub const MIGRATION_PARAMS: Item<MigrationParams> = Item::new("migration_params");

#[cw_serde]
#[serde(untagged)]
pub enum MigrationMsgs {
    DaoProposalSingle(dao_proposal_single::msg::MigrateMsg),
    DaoCore(dao_core::msg::MigrateMsg),
    DaoVotingCw4(dao_voting_cw4::msg::MigrateMsg),
    Cw20Stake(cw20_stake::msg::MigrateMsg),
    DaoVotingCw20Staked(dao_voting_cw20_staked::msg::MigrateMsg),
}

pub struct CodeIdPair {
    pub v1_code_id: u64,
    pub v2_code_id: u64,
    pub migrate_msg: MigrationMsgs,
}

impl CodeIdPair {
    pub fn new(v1_code_id: u64, v2_code_id: u64, migrate_msg: MigrationMsgs) -> CodeIdPair {
        CodeIdPair {
            v1_code_id,
            v2_code_id,
            migrate_msg,
        }
    }
}
