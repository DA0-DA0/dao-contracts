use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128};
use dao_voting::veto::VetoConfig;

use crate::ContractError;

#[cw_serde]
pub struct V1CodeIds {
    pub proposal_single: u64,
    pub cw4_voting: u64,
    pub cw20_stake: u64,
    pub cw20_staked_balances_voting: u64,
}

impl V1CodeIds {
    pub fn to(self) -> dao_interface::migrate_msg::V1CodeIds {
        dao_interface::migrate_msg::V1CodeIds {
            proposal_single: self.proposal_single,
            cw4_voting: self.cw4_voting,
            cw20_stake: self.cw20_stake,
            cw20_staked_balances_voting: self.cw20_staked_balances_voting,
        }
    }
}

#[cw_serde]
pub struct V2CodeIds {
    pub proposal_single: u64,
    pub cw4_voting: u64,
    pub cw20_stake: u64,
    pub cw20_staked_balances_voting: u64,
}

impl V2CodeIds {
    pub fn to(self) -> dao_interface::migrate_msg::V2CodeIds {
        dao_interface::migrate_msg::V2CodeIds {
            proposal_single: self.proposal_single,
            cw4_voting: self.cw4_voting,
            cw20_stake: self.cw20_stake,
            cw20_staked_balances_voting: self.cw20_staked_balances_voting,
        }
    }
}

/// The params we need to provide for migration msgs
#[cw_serde]
pub struct ProposalParams {
    pub close_proposal_on_execution_failure: bool,
    pub pre_propose_info: dao_voting::pre_propose::PreProposeInfo,
    pub veto: Option<VetoConfig>,
}

#[cw_serde]
pub struct MigrationParams {
    // General
    /// Rather or not to migrate the stake_cw20 contract and its
    /// manager. If this is not set to true and a stake_cw20
    /// contract is detected in the DAO's configuration the
    /// migration will be aborted.
    pub migrate_stake_cw20_manager: Option<bool>,
    /// List of (address, ProposalParams) where `address` is an
    /// address of a proposal module currently part of the DAO.
    pub proposal_params: Vec<(String, ProposalParams)>,
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
#[derive(Clone)]
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
    pub voting: Option<Addr>,
    pub proposals: Vec<Addr>,
}

impl ModulesAddrs {
    pub fn verify(&self) -> Result<(), ContractError> {
        if self.voting.is_none() {
            return Err(ContractError::VotingModuleNotFound);
        }

        if self.proposals.is_empty() {
            return Err(ContractError::DaoProposalSingleNotFound);
        }
        Ok(())
    }
}

// Test helper types

pub struct SingleProposalData {
    pub proposer: Addr,
    pub start_height: u64,
}

/// Data we use to test after migration (it is set before migration)
#[cw_serde]
pub struct TestState {
    pub proposal_counts: Vec<u64>,
    pub proposals: Vec<dao_proposal_single::proposal::SingleChoiceProposal>,
    pub total_voting_power: Uint128,
    /// This is the voting power of the proposer of the sample proposal
    pub single_voting_power: Uint128,
}
