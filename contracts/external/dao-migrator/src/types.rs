use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Deps, Uint128};
use dao_core::state::ProposalModule;

use crate::ContractError;

#[cw_serde]
pub struct V1CodeIds {
    pub proposal_single: u64,
    pub cw4_voting: u64,
    pub cw20_stake: u64,
    pub cw20_staked_balances_voting: u64,
}

#[cw_serde]
pub struct V2CodeIds {
    pub proposal_single: u64,
    pub cw4_voting: u64,
    pub cw20_stake: u64,
    pub cw20_staked_balances_voting: u64,
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
}

/// Wrapper enum that helps us to hold different types of migration msgs
#[cw_serde]
#[serde(untagged)]
pub enum MigrationMsgs {
    DaoProposalSingle(dao_proposal_single::msg::MigrateMsg),
    DaoVotingCw4(dao_voting_cw4::msg::MigrateMsg),
    Cw20Stake(cw20_stake::msg::MigrateMsg),
    DaoVotingCw20Staked(dao_voting_cw20_staked::msg::MigrateMsg),
}

/// Module data we need for migrations and tests.
pub struct CodeIdPair {
    /// The code id used in V1 module
    pub v1_code_id: u64,
    /// The new code id used in V2
    pub v2_code_id: u64,
    /// The migration msg of the module
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

/// Hold module addresses to do queries on
#[cw_serde]
#[derive(Default)]
pub struct ModulesAddrs {
    pub voting: Option<String>,
    pub proposals: Vec<String>,
}

impl ModulesAddrs {
    pub fn verify(&self, deps: Deps) -> Result<(), ContractError> {
        if self.voting.is_none() {
            return Err(ContractError::VotingModuleNotFound);
        }

        deps.api.addr_validate(self.voting.as_ref().unwrap())?;

        if self.proposals.is_empty() {
            return Err(ContractError::DaoProposalSingleNotFound);
        }

        // Verify proposal vec are addresses
        self.proposals
            .iter()
            .find_map(|x| {
                if let Err(err) = deps.api.addr_validate(x.as_ref()) {
                    Some(Err(err))
                } else {
                    None
                }
            })
            .unwrap_or(Ok(()))?;
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
    // pub core_dump_state: TestCoreDumpState,
    // pub core_items: Vec<(String, String)>,
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
