use std::ops::{Deref, DerefMut};

use cosmwasm_std::{Addr, Decimal};
use dao_testing::{Cw721TestDao, DaoTestingSuite, InitialNft};

use super::base::DaoVoteDelegationTestingSuiteBase;

pub struct Cw721DaoVoteDelegationTestingSuite {
    /// base testing suite that we're extending
    pub base: DaoVoteDelegationTestingSuiteBase,

    /// cw721 voting DAO
    pub dao: Cw721TestDao,

    /// members of the dao
    pub members: Vec<InitialNft>,
}

// allow direct access to base testing suite methods
impl Deref for Cw721DaoVoteDelegationTestingSuite {
    type Target = DaoVoteDelegationTestingSuiteBase;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

// allow direct access to base testing suite methods
impl DerefMut for Cw721DaoVoteDelegationTestingSuite {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

// CONSTRUCTOR
impl Cw721DaoVoteDelegationTestingSuite {
    pub fn new() -> Self {
        let mut base = DaoVoteDelegationTestingSuiteBase::new();
        let mut suite = base.cw721();

        let dao = suite.dao();
        let members = suite.initial_nfts.clone();

        base.dao_core_addr = dao.core_addr.clone();

        Self { base, dao, members }
    }

    pub fn with_vp_cap_percent(mut self, vp_cap_percent: Decimal) -> Self {
        self.vp_cap_percent = Some(vp_cap_percent);
        self
    }

    pub fn with_delegation_validity_blocks(mut self, delegation_validity_blocks: u64) -> Self {
        self.delegation_validity_blocks = Some(delegation_validity_blocks);
        self
    }

    pub fn with_max_delegations(mut self, max_delegations: u64) -> Self {
        self.max_delegations = Some(max_delegations);
        self
    }

    pub fn build(mut self) -> Self {
        let code_id = self.delegation_code_id;
        let core_addr = self.dao.core_addr.clone();
        let vp_cap_percent = self.vp_cap_percent;
        let delegation_validity_blocks = self.delegation_validity_blocks;
        let max_delegations = self.max_delegations;
        let voting_module_addr = self.dao.voting_module_addr.to_string();

        self.delegation_addr = self.instantiate(
            code_id,
            &core_addr,
            &crate::msg::InstantiateMsg {
                dao: None,
                vp_hook_callers: Some(vec![voting_module_addr]),
                sync_proposal_modules: None,
                vp_cap_percent,
                delegation_validity_blocks,
                max_delegations,
            },
            &[],
            "delegation",
            Some(core_addr.to_string()),
        );

        self.setup_delegation_module();

        self
    }

    /// set up delegation module by adding necessary hooks and adding it to the
    /// proposal modules
    pub fn setup_delegation_module(&mut self) {
        let dao = self.dao.clone();
        let delegation_addr = self.delegation_addr.to_string();

        // add voting power changed hook to cw721-group
        self.execute_smart_ok(
            &dao.core_addr,
            &dao.voting_module_addr,
            &dao_voting_cw721_staked::msg::ExecuteMsg::AddHook {
                addr: delegation_addr.clone(),
            },
            &[],
        );

        // add vote hook to all proposal modules
        self.add_vote_hook(&dao, &delegation_addr);

        // set the delegation module for all proposal modules
        self.set_delegation_module(&dao, &delegation_addr);

        // ensure delegation modules are set
        dao.proposal_modules.iter().for_each(|(_, module)| {
            let delegation_module = self
                .querier()
                .query_wasm_smart::<Option<Addr>>(
                    module,
                    &dao_proposal_single::msg::QueryMsg::DelegationModule {},
                )
                .unwrap()
                .unwrap();

            assert_eq!(delegation_module, Addr::unchecked(delegation_addr.clone()));
        });
    }
}
