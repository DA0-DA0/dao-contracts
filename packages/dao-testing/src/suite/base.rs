use cosmwasm_std::{to_json_binary, Addr, Empty, QuerierWrapper};
use cw_multi_test::{App, Executor};
use cw_utils::Duration;

use super::*;
use crate::contracts::*;

pub struct TestDao {
    pub core: Addr,
    pub voting_module: Addr,
    pub proposal_modules: Vec<dao_interface::state::ProposalModule>,
}

pub struct DaoTestingSuiteBase {
    pub app: App,

    // Code IDs
    // DAO stuff
    pub core_id: u64,
    pub admin_factory_id: u64,
    pub proposal_single_id: u64,
    pub proposal_multiple_id: u64,
    pub proposal_sudo_id: u64,
    pub pre_propose_approval_single_id: u64,
    pub pre_propose_single_id: u64,
    pub pre_propose_multiple_id: u64,
    pub pre_propose_approver_id: u64,
    pub voting_cw4_id: u64,
    pub voting_cw20_staked_id: u64,
    pub voting_cw20_balance_id: u64,
    pub voting_cw721_staked_id: u64,
    pub voting_token_staked_id: u64,
    pub cw20_stake_id: u64,
    pub rewards_distributor_id: u64,
    // External stuff
    pub cw4_group_id: u64,
    pub cw20_base_id: u64,
    pub cw721_base_id: u64,

    // Addresses
    pub admin_factory_addr: Addr,
}

pub trait DaoTestingSuite {
    /// get the testing suite base
    fn base(&self) -> &DaoTestingSuiteBase;

    /// get the mutable testing suite base
    fn base_mut(&mut self) -> &mut DaoTestingSuiteBase;

    /// get the voting module info to instantiate the DAO with
    fn get_voting_module_info(&self) -> dao_interface::state::ModuleInstantiateInfo;

    /// build the DAO
    fn dao(&mut self) -> TestDao {
        let voting_module_info = self.get_voting_module_info();

        let proposal_module_infos =
            vec![dao_interface::state::ModuleInstantiateInfo {
            code_id: self.base().proposal_single_id,
            msg: to_json_binary(&dao_proposal_single::msg::InstantiateMsg {
                threshold: dao_voting::threshold::Threshold::AbsolutePercentage {
                    percentage: dao_voting::threshold::PercentageThreshold::Majority {},
                },
                max_voting_period: Duration::Height(10),
                min_voting_period: None,
                only_members_execute: true,
                allow_revoting: false,
                pre_propose_info: dao_voting::pre_propose::PreProposeInfo::ModuleMayPropose {
                    info: dao_interface::state::ModuleInstantiateInfo {
                        code_id: self.base().pre_propose_single_id,
                        msg: to_json_binary(&dao_pre_propose_single::InstantiateMsg {
                            deposit_info: None,
                            submission_policy:
                                dao_voting::pre_propose::PreProposeSubmissionPolicy::Specific {
                                    dao_members: true,
                                    allowlist: vec![],
                                    denylist: vec![],
                                },
                            extension: Empty {},
                        })
                        .unwrap(),
                        admin: Some(dao_interface::state::Admin::CoreModule {}),
                        funds: vec![],
                        label: "single choice pre-propose module".to_string(),
                    },
                },
                close_proposal_on_execution_failure: true,
                veto: None,
            })
            .unwrap(),
            admin: Some(dao_interface::state::Admin::CoreModule {}),
            funds: vec![],
            label: "single choice proposal module".to_string(),
        },
        dao_interface::state::ModuleInstantiateInfo {
            code_id: self.base().proposal_multiple_id,
            msg: to_json_binary(&dao_proposal_multiple::msg::InstantiateMsg {
                voting_strategy: dao_voting::multiple_choice::VotingStrategy::SingleChoice {
                    quorum: dao_voting::threshold::PercentageThreshold::Majority {},
                },
                max_voting_period: Duration::Height(10),
                min_voting_period: None,
                only_members_execute: true,
                allow_revoting: false,
                pre_propose_info: dao_voting::pre_propose::PreProposeInfo::ModuleMayPropose {
                    info: dao_interface::state::ModuleInstantiateInfo {
                        code_id: self.base().pre_propose_multiple_id,
                        msg: to_json_binary(&dao_pre_propose_multiple::InstantiateMsg {
                            deposit_info: None,
                            submission_policy:
                                dao_voting::pre_propose::PreProposeSubmissionPolicy::Specific {
                                    dao_members: true,
                                    allowlist: vec![],
                                    denylist: vec![],
                                },
                            extension: Empty {},
                        })
                        .unwrap(),
                        admin: Some(dao_interface::state::Admin::CoreModule {}),
                        funds: vec![],
                        label: "multiple choice pre-propose module".to_string(),
                    },
                },
                close_proposal_on_execution_failure: true,
                veto: None,
            })
            .unwrap(),
            admin: Some(dao_interface::state::Admin::CoreModule {}),
            funds: vec![],
            label: "multiple choice proposal module".to_string(),
        }];

        return self
            .base_mut()
            .build(voting_module_info, proposal_module_infos);
    }

    /// get the app querier
    fn querier(&self) -> QuerierWrapper<'_> {
        self.base().app.wrap()
    }
}

impl DaoTestingSuiteBase {
    pub fn new() -> Self {
        let mut app = App::default();

        let core_id = app.store_code(dao_dao_core_contract());
        let admin_factory_id = app.store_code(cw_admin_factory_contract());
        let proposal_single_id = app.store_code(dao_proposal_single_contract());
        let proposal_multiple_id = app.store_code(dao_proposal_multiple_contract());
        let proposal_sudo_id = app.store_code(dao_proposal_sudo_contract());
        let pre_propose_approval_single_id =
            app.store_code(dao_pre_propose_approval_single_contract());
        let pre_propose_single_id = app.store_code(dao_pre_propose_single_contract());
        let pre_propose_multiple_id = app.store_code(dao_pre_propose_multiple_contract());
        let pre_propose_approver_id = app.store_code(dao_pre_propose_approver_contract());
        let voting_cw4_id = app.store_code(dao_voting_cw4_contract());
        let voting_cw20_staked_id = app.store_code(dao_voting_cw20_staked_contract());
        let voting_cw20_balance_id = app.store_code(dao_voting_cw20_balance_contract());
        let voting_cw721_staked_id = app.store_code(dao_voting_cw721_staked_contract());
        let voting_token_staked_id = app.store_code(dao_voting_token_staked_contract());
        let cw20_stake_id = app.store_code(cw20_stake_contract());
        let rewards_distributor_id = app.store_code(dao_rewards_distributor_contract());

        let cw4_group_id = app.store_code(cw4_group_contract());
        let cw20_base_id = app.store_code(cw20_base_contract());
        let cw721_base_id = app.store_code(cw721_base_contract());

        let admin_factory_addr = app
            .instantiate_contract(
                admin_factory_id,
                Addr::unchecked(CREATOR),
                &cw_admin_factory::msg::InstantiateMsg { admin: None },
                &[],
                "admin factory",
                None,
            )
            .unwrap();

        Self {
            app,

            core_id,
            admin_factory_id,
            proposal_single_id,
            proposal_multiple_id,
            proposal_sudo_id,
            pre_propose_approval_single_id,
            pre_propose_single_id,
            pre_propose_multiple_id,
            pre_propose_approver_id,
            voting_cw4_id,
            voting_cw20_staked_id,
            voting_cw20_balance_id,
            voting_cw721_staked_id,
            voting_token_staked_id,
            cw20_stake_id,
            rewards_distributor_id,
            cw4_group_id,
            cw20_base_id,
            cw721_base_id,

            admin_factory_addr,
        }
    }
}

impl DaoTestingSuiteBase {
    pub fn build(
        &mut self,
        voting_module_instantiate_info: dao_interface::state::ModuleInstantiateInfo,
        proposal_modules_instantiate_info: Vec<dao_interface::state::ModuleInstantiateInfo>,
    ) -> TestDao {
        let init = dao_interface::msg::InstantiateMsg {
            admin: None,
            name: "DAO DAO".to_string(),
            description: "A DAO that builds DAOs.".to_string(),
            image_url: None,
            automatically_add_cw20s: false,
            automatically_add_cw721s: false,
            voting_module_instantiate_info,
            proposal_modules_instantiate_info,
            initial_items: None,
            dao_uri: None,
        };

        let res = self
            .app
            .execute_contract(
                Addr::unchecked(CREATOR),
                self.admin_factory_addr.clone(),
                &cw_admin_factory::msg::ExecuteMsg::InstantiateContractWithSelfAdmin {
                    instantiate_msg: to_json_binary(&init).unwrap(),
                    code_id: self.core_id,
                    label: "DAO DAO".to_string(),
                },
                &[],
            )
            .unwrap();

        // get core address from the instantiate event
        let instantiate_event = &res.events[2];
        assert_eq!(instantiate_event.ty, "instantiate");
        let core = Addr::unchecked(instantiate_event.attributes[0].value.clone());

        // get voting module address
        let voting_module: Addr = self
            .app
            .wrap()
            .query_wasm_smart(&core, &dao_interface::msg::QueryMsg::VotingModule {})
            .unwrap();

        // get proposal modules
        let proposal_modules: Vec<dao_interface::state::ProposalModule> = self
            .app
            .wrap()
            .query_wasm_smart(
                &core,
                &dao_interface::msg::QueryMsg::ProposalModules {
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap();

        TestDao {
            core,
            voting_module,
            proposal_modules,
        }
    }

    pub fn cw4(&mut self) -> DaoTestingSuiteCw4 {
        DaoTestingSuiteCw4::new(self)
    }
}

#[cfg(test)]
mod test {
    use cosmwasm_std::Uint128;

    use super::*;

    #[test]
    fn dao_testing_suite_cw4() {
        let mut suite = DaoTestingSuiteBase::new();
        let mut suite = suite.cw4();
        let cw4_dao = suite.dao();

        let voting_module: Addr = suite
            .querier()
            .query_wasm_smart(
                &cw4_dao.core,
                &dao_interface::msg::QueryMsg::VotingModule {},
            )
            .unwrap();
        assert_eq!(voting_module, cw4_dao.voting_module);

        let proposal_modules: Vec<dao_interface::state::ProposalModule> = suite
            .querier()
            .query_wasm_smart(
                &cw4_dao.core,
                &dao_interface::msg::QueryMsg::ProposalModules {
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap();
        assert_eq!(proposal_modules.len(), 2);

        let total_weight: dao_interface::voting::TotalPowerAtHeightResponse = suite
            .querier()
            .query_wasm_smart(
                &cw4_dao.core,
                &dao_interface::msg::QueryMsg::TotalPowerAtHeight { height: None },
            )
            .unwrap();
        assert_eq!(
            total_weight.power,
            suite
                .members
                .iter()
                .fold(Uint128::zero(), |acc, m| acc + Uint128::from(m.weight))
        );
    }
}
