use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128};
use dao_core::state::ProposalModule;

use crate::ContractError;

#[derive(Copy, Clone)]
pub enum V1CodeIds {
    Core = 452,
    ProposalSingle = 453,
    Cw4Voting = 450,
    Cw20Stake = 449,
    Cw20StakedBalancesVoting = 451,
}

#[derive(Copy, Clone)]
pub enum V2CodeIds {
    Core = 3457,
    ProposalSingle = 3463,
    Cw4Voting = 3465,
    Cw20Stake = 3454,
    Cw20StakedBalancesVoting = 3464,
}

/// The params we need to provide for migration msgs
#[cw_serde]
pub struct MigrationParams {
    // General
    /// Rather or not to migrate the stake_cw20 contract and its
    /// manager. If this is not set to true and a stake_cw20
    /// contract is detected in the DAO's configuration the
    /// migration will be aborted.
    pub migrate_stake_cw20_manager: Option<bool>,
    // dao_proposal_single
    pub close_proposal_on_execution_failure: bool,
    pub pre_propose_info: dao_voting::pre_propose::PreProposeInfo,
    // dao_core
    pub dao_uri: Option<String>,
}

/// Wrapper enum that helps us to hold different types of migration msgs
#[cw_serde]
#[serde(untagged)]
pub enum MigrationMsgs {
    DaoProposalSingle(dao_proposal_single::msg::MigrateMsg),
    DaoCore(dao_core::msg::MigrateMsg),
    DaoVotingCw4(dao_voting_cw4::msg::MigrateMsg),
    Cw20Stake(cw20_stake::msg::MigrateMsg),
    DaoVotingCw20Staked(dao_voting_cw20_staked::msg::MigrateMsg),
}

/// Module data we need for migrations and tests.
pub struct CodeIdPair {
    /// The code id used in V1 module
    pub v1_code_id: V1CodeIds,
    /// The new code id used in V2
    pub v2_code_id: V2CodeIds,
    /// The migration msg of the module
    pub migrate_msg: MigrationMsgs,
}

impl CodeIdPair {
    pub fn new(v1_code_id: V1CodeIds, v2_code_id: V2CodeIds, migrate_msg: MigrationMsgs) -> CodeIdPair {
        CodeIdPair {
            v1_code_id,
            v2_code_id,
            migrate_msg,
        }
    }
}

/// Hold module addresses to do queries on
#[cw_serde]
pub struct ModulesAddrs {
    pub core: Option<Addr>,
    pub proposals: Vec<Addr>,
}

impl ModulesAddrs {
    pub fn new() -> ModulesAddrs {
        ModulesAddrs {
            core: None,
            proposals: vec![],
        }
    }
    pub fn verify(&self) -> Result<(), ContractError> {
        if self.core.is_none() {
            return Err(ContractError::DaoCoreNotFound);
        }

        if self.proposals.len() == 0 {
            return Err(ContractError::DaoProposalSingleNotFound);
        }
        Ok(())
    }
}

// Test helper types

/// Data we use to test after migration (it is set before migration)

pub struct SingleProposalData {
    pub proposer: Addr,
    pub start_height: u64,
}

#[cw_serde]
pub struct TestState {
    pub core_dump_state: TestCoreDumpState,
    pub core_items: Vec<(String, String)>,
    pub proposal_counts: Vec<u64>,
    pub proposals: Vec<dao_proposal_single::proposal::SingleChoiceProposal>,
    pub total_voting_power: Uint128,
    pub single_voting_power: Uint128,
}

#[cw_serde]
pub struct TestCoreDumpState {
    pub proposal_modules: Vec<ProposalModule>,
    pub voting_module: Addr,
    pub total_proposal_module_count: u32,
}
