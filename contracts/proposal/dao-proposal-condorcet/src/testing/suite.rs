use cosmwasm_std::{to_binary, Addr, CosmosMsg, Decimal};
use cw_multi_test::{next_block, App, Executor};
use cw_utils::Duration;
use dao_interface::{voting::InfoResponse, Admin, ModuleInstantiateInfo};
use dao_testing::contracts::{
    cw4_group_contract, dao_core_contract, dao_voting_cw4_contract, proposal_condorcet_contract,
};
use dao_voting::threshold::PercentageThreshold;

use crate::{
    config::{Config, UncheckedConfig},
    contract::{CONTRACT_NAME, CONTRACT_VERSION},
    msg::{Choice, ExecuteMsg, InstantiateMsg, QueryMsg},
    proposal::ProposalResponse,
};

pub(crate) struct Suite {
    app: App,
    condorcet: Addr,
    core: Addr,

    pub sender: Addr,
}

pub(crate) struct SuiteBuilder {
    pub instantiate: InstantiateMsg,
}

impl Default for SuiteBuilder {
    fn default() -> Self {
        Self {
            instantiate: UncheckedConfig {
                quorum: PercentageThreshold::Percent(Decimal::percent(15)),
                voting_period: Duration::Time(60 * 60 * 24 * 7),
                min_voting_period: Some(Duration::Time(60 * 60 * 24)),
                close_proposals_on_execution_failure: true,
            },
        }
    }
}

impl SuiteBuilder {
    pub fn with_config(instantiate: UncheckedConfig) -> Self {
        Self { instantiate }
    }

    pub fn build(self) -> Suite {
        let sender = Addr::unchecked("sender");

        let mut app = App::default();
        let condorcet_id = app.store_code(proposal_condorcet_contract());
        let core_id = app.store_code(dao_core_contract());
        let cw4_id = app.store_code(cw4_group_contract());
        let cw4_voting_id = app.store_code(dao_voting_cw4_contract());

        let core_instantiate = dao_core::msg::InstantiateMsg {
            admin: None,
            name: "core module".to_string(),
            description: "core module".to_string(),
            image_url: Some("https://moonphase.is/image.svg".to_string()),
            automatically_add_cw20s: false,
            automatically_add_cw721s: false,
            voting_module_instantiate_info: ModuleInstantiateInfo {
                code_id: cw4_voting_id,
                msg: to_binary(&dao_voting_cw4::msg::InstantiateMsg {
                    cw4_group_code_id: cw4_id,
                    initial_members: vec![cw4::Member {
                        addr: sender.to_string(),
                        weight: 10,
                    }],
                })
                .unwrap(),
                admin: Some(Admin::CoreModule {}),
                label: "voting module".to_string(),
            },
            proposal_modules_instantiate_info: vec![ModuleInstantiateInfo {
                code_id: condorcet_id,
                msg: to_binary(&self.instantiate).unwrap(),
                admin: Some(Admin::CoreModule {}),
                label: "condorcet module".to_string(),
            }],
            initial_items: None,
            dao_uri: None,
        };
        let core = app
            .instantiate_contract(
                core_id,
                sender.clone(),
                &core_instantiate,
                &[],
                "core module".to_string(),
                None,
            )
            .unwrap();
        let condorcet: Vec<dao_core::state::ProposalModule> = app
            .wrap()
            .query_wasm_smart(
                &core,
                &dao_core::msg::QueryMsg::ProposalModules {
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap();
        let condorcet = condorcet.into_iter().next().unwrap().address;

        app.update_block(next_block);

        let suite = Suite {
            app,
            sender,
            condorcet,
            core,
        };

        let dao = suite.query_dao();
        assert_eq!(dao, suite.core);
        let next_id = suite.query_next_proposal_id();
        assert_eq!(next_id, 1);
        let info = suite.query_info();
        assert_eq!(info.info.version, CONTRACT_VERSION);
        assert_eq!(info.info.contract, CONTRACT_NAME);

        suite
    }
}

impl Suite {
    pub fn block_height(&self) -> u64 {
        self.app.block_info().height
    }
}

// query
impl Suite {
    pub fn query_config(&self) -> Config {
        self.app
            .wrap()
            .query_wasm_smart(&self.condorcet, &QueryMsg::Config {})
            .unwrap()
    }

    pub fn query_proposal(&self, id: u32) -> ProposalResponse {
        self.app
            .wrap()
            .query_wasm_smart(&self.condorcet, &QueryMsg::Proposal { id })
            .unwrap()
    }

    pub fn query_next_proposal_id(&self) -> u32 {
        self.app
            .wrap()
            .query_wasm_smart(&self.condorcet, &QueryMsg::NextProposalId {})
            .unwrap()
    }

    pub fn query_dao(&self) -> Addr {
        self.app
            .wrap()
            .query_wasm_smart(&self.condorcet, &QueryMsg::Dao {})
            .unwrap()
    }

    pub fn query_info(&self) -> InfoResponse {
        self.app
            .wrap()
            .query_wasm_smart(&self.condorcet, &QueryMsg::Info {})
            .unwrap()
    }
}

// execute
impl Suite {
    pub fn propose<S: Into<String>>(
        &mut self,
        sender: S,
        choices: Vec<Vec<CosmosMsg>>,
    ) -> anyhow::Result<u32> {
        let id = self.query_next_proposal_id();
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.condorcet.clone(),
            &ExecuteMsg::Propose {
                choices: choices.into_iter().map(|msgs| Choice { msgs }).collect(),
            },
            &[],
        )?;
        Ok(id)
    }
}
