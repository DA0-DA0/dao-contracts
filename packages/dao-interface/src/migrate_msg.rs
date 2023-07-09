//! types used for migrating modules of the DAO with migrating core
//! copyo of the types from dao-migrator contract.

use cosmwasm_schema::cw_serde;

use crate::query::SubDao;
use crate::state::ModuleInstantiateInfo;

#[cw_serde]
pub struct MigrateParams {
    pub migrator_code_id: u64,
    pub params: MigrateV1ToV2,
}

#[cw_serde]
pub struct MigrateV1ToV2 {
    pub sub_daos: Vec<SubDao>,
    pub migration_params: MigrationModuleParams,
    pub v1_code_ids: V1CodeIds,
    pub v2_code_ids: V2CodeIds,
}

// code ids for the v1 contracts
#[cw_serde]
pub struct V1CodeIds {
    pub proposal_single: u64,
    pub cw4_voting: u64,
    pub cw20_stake: u64,
    pub cw20_staked_balances_voting: u64,
}

// code ids for the new contracts
#[cw_serde]
pub struct V2CodeIds {
    pub proposal_single: u64,
    pub cw4_voting: u64,
    pub cw20_stake: u64,
    pub cw20_staked_balances_voting: u64,
}

/// The params we need to provide for migration msgs
#[cw_serde]
pub struct ProposalParams {
    pub close_proposal_on_execution_failure: bool,
    pub pre_propose_info: PreProposeInfo,
}

#[cw_serde]
pub struct MigrationModuleParams {
    // General
    /// Rather or not to migrate the stake_cw20 contract and its
    /// manager. If this is not set to true and a stake_cw20
    /// contract is detected in the DAO's configuration the
    /// migration will be aborted.
    pub migrate_stake_cw20_manager: Option<bool>,
    // dao_proposal_single
    pub proposal_params: Vec<(String, ProposalParams)>,
}

#[cw_serde]
pub enum PreProposeInfo {
    /// Anyone may create a proposal free of charge.
    AnyoneMayPropose {},
    /// The module specified in INFO has exclusive rights to proposal
    /// creation.
    ModuleMayPropose { info: ModuleInstantiateInfo },
}
