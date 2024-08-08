use cosmwasm_std::{coin, coins, to_json_binary, Decimal, Uint128};
use cw4::Member;
use cw_orch::{anyhow, prelude::*};
use dao_cw_orch::{DaoDaoCore, DaoProposalSingle, DaoVotingCw4};
use dao_gauge_adapter::contract::ExecuteMsg as AdapterExecuteMsg;
use dao_interface::{
    msg::ExecuteMsg as CoreExecuteMsg,
    state::{Admin, ModuleInstantiateInfo},
};
use dao_voting_cw4::msg::QueryMsgFns as _;
use gauge_orchestrator::{
    msg::{
        ExecuteMsg as GaugeExecuteMsg, ExecuteMsgFns as OrchExecuteMsgFns, GaugeConfig,
        GaugeMigrationConfig, GaugeResponse, MigrateMsg, QueryMsgFns as GaugeOrchQueryMsgFns,
    },
    state::Vote as GaugeVote,
    ContractError,
};

use crate::{
    tests::{daos::voting::dao_cw4_voting_template, gauges::helpers::EPOCH, PREFIX},
    GaugeSuite,
};

/// cw4 voting dao with gauges
pub struct DaoDaoCw4Gauge<Chain> {
    pub dao_core: DaoDaoCore<Chain>,
    pub prop_single: DaoProposalSingle<Chain>,
    pub cw4_vote: DaoVotingCw4<Chain>,
    pub gauge_suite: GaugeSuite<Chain>,
    pub cw4_group: Option<u64>,
}

impl<Chain: CwEnv> DaoDaoCw4Gauge<Chain> {
    pub fn new(chain: Chain) -> DaoDaoCw4Gauge<Chain> {
        DaoDaoCw4Gauge::<Chain> {
            dao_core: DaoDaoCore::new("dao_dao_core", chain.clone()),
            prop_single: DaoProposalSingle::new("dao_prop_single", chain.clone()),
            cw4_vote: DaoVotingCw4::new("dao_cw4_voting", chain.clone()),
            gauge_suite: GaugeSuite::new(chain.clone()),
            cw4_group: None,
        }
    }
    pub fn upload(&self) -> Result<(), CwOrchError> {
        self.dao_core.upload()?;
        self.prop_single.upload()?;
        self.cw4_vote.upload()?;
        self.gauge_suite.upload()?;
        Ok(())
    }
}
impl DaoDaoCw4Gauge<MockBech32> {
    pub fn upload_with_cw4(&mut self, mock: MockBech32) -> Result<u64, CwOrchError> {
        self.upload()?;
        // also upload cw4 group
        let cw4 = mock
            .upload_custom(
                "cw4",
                Box::new(ContractWrapper::new_with_empty(
                    cw4_group::contract::execute,
                    cw4_group::contract::instantiate,
                    cw4_group::contract::query,
                )),
            )?
            .uploaded_code_id()?;
        self.cw4_group = Some(cw4);
        Ok(cw4)
    }
    pub fn custom_gauge_setup(
        &mut self,
        mock: MockBech32,
        dao_members: Vec<Coin>,
        options: &[&str],
    ) -> Result<(), CwOrchError> {
        let init_members = self.custom_initial_members(dao_members)?;
        // create dao
        let dao_modules = dao_cw4_voting_template(mock.clone(), self, init_members)?;
        // set contracts to cw-orch state
        self.set_dao_module_addrs(dao_modules[1].clone(), dao_modules[0].clone())?;
        // create gauge adapter
        let gauge_config = self.init_adapter(&options)?;
        let adapter = Addr::unchecked(gauge_config.adapter.clone());
        // create orchestrator & add to DAO
        let gauge = self.add_gauge_to_dao(mock.clone(), vec![gauge_config])?;
        // set gauges to cw-orch suite
        self.set_gauge_suite_addrs(gauge.clone(), adapter)?;
        mock.add_balance(&self.dao_core.address()?, coins(10000, "ujuno"))?;

        Ok(())
    }
    pub fn default_gauge_setup(&mut self, mock: MockBech32) -> Result<(), CwOrchError> {
        let voter1 = mock.addr_make("voter1");
        let voter2 = mock.addr_make("voter2");

        let init_members = self.default_inital_members(mock.clone())?;
        // create dao
        let dao_modules = dao_cw4_voting_template(mock.clone(), self, init_members)?;
        // set contracts to cw-orch state
        self.set_dao_module_addrs(dao_modules[1].clone(), dao_modules[0].clone())?;
        let dao_addr = self.dao_core.addr_str()?;
        // create gauge adapter
        let default_options = vec![voter1.as_str(), voter2.as_str(), &dao_addr];
        let gauge_config = self.init_adapter(&default_options)?;
        let adapter = Addr::unchecked(gauge_config.adapter.clone());
        // create orchestrator & add to DAO
        let gauge = self.add_gauge_to_dao(mock.clone(), vec![gauge_config])?;
        // set gauges to cw-orch suite
        self.set_gauge_suite_addrs(gauge.clone(), adapter)?;
        mock.add_balance(&self.dao_core.address()?, coins(10000, "ujuno"))?;

        Ok(())
    }
    pub fn set_dao_module_addrs(&mut self, vote: Addr, proposal: Addr) -> anyhow::Result<()> {
        self.cw4_vote.set_default_address(&vote);
        self.prop_single.set_default_address(&proposal);
        Ok(())
    }
    pub fn set_gauge_suite_addrs(&mut self, gauge: Addr, adapter: Addr) -> anyhow::Result<()> {
        self.gauge_suite.orchestrator.set_default_address(&gauge);
        self.gauge_suite.adapter.set_default_address(&adapter);
        Ok(())
    }

    pub fn custom_initial_members(&self, members: Vec<Coin>) -> anyhow::Result<Vec<Member>> {
        let mut res: Vec<Member> = vec![];
        for member in members {
            res.push(Member {
                addr: member.denom,
                weight: member.amount.u128().try_into().unwrap(),
            })
        }

        Ok(res)
    }
    pub fn default_inital_members(&self, mock: MockBech32) -> anyhow::Result<Vec<Member>> {
        let mut res: Vec<Member> = vec![];
        let members = vec![
            coin(100, &mock.sender.to_string()),
            coin(100, mock.addr_make("voter1")),
            coin(100, mock.addr_make("voter2")),
            coin(600, mock.addr_make("voter3")),
            coin(120, mock.addr_make("voter4")),
            coin(130, mock.addr_make("voter5")),
            coin(140, mock.addr_make("voter6")),
            coin(150, mock.addr_make("voter7")),
        ];

        for member in members {
            res.push(Member {
                addr: member.denom,
                weight: member.amount.u128().try_into().unwrap(),
            })
        }
        Ok(res)
    }

    pub fn init_adapter(&self, options: &[&str]) -> anyhow::Result<GaugeConfig> {
        // init adapter
        let adapter = self.gauge_suite.test_adapter.instantiate(
            &dao_gauge_adapter::contract::InstantiateMsg {
                options: options.iter().map(|s| s.to_string()).collect(),
                to_distribute: coin(1000, "ujuno"),
            },
            Some(&self.dao_core.address()?),
            None,
        )?;
        Ok(GaugeConfig {
            title: "default-gauge".to_owned(),
            adapter: adapter.instantiated_contract_address()?.to_string(),
            epoch_size: EPOCH,
            min_percent_selected: Some(Decimal::percent(5)),
            max_options_selected: 10,
            max_available_percentage: None,
            reset_epoch: None,
            total_epochs: None,
        })
    }

    pub fn init_gauge(&self) -> anyhow::Result<Addr> {
        // init gauge
        Ok(self
            .gauge_suite
            .orchestrator
            .instantiate(
                &gauge_orchestrator::msg::InstantiateMsg {
                    voting_powers: self.cw4_vote.addr_str()?,
                    hook_caller: self.cw4_vote.addr_str()?,
                    owner: self.dao_core.addr_str()?,
                    gauges: None,
                },
                Some(&self.dao_core.address()?),
                None,
            )?
            .instantiated_contract_address()?)
    }

    pub fn add_gauge_to_dao(
        &self,
        mock: MockBech32,
        gauge_config: Vec<GaugeConfig>,
    ) -> anyhow::Result<Addr> {
        let dao_addr = self.dao_core.address()?;
        let cw4_addr = self.cw4_vote.address()?;

        let gauge = mock
            .call_as(&dao_addr)
            .execute(
                &CoreExecuteMsg::UpdateProposalModules {
                    to_add: vec![ModuleInstantiateInfo {
                        code_id: self.gauge_suite.orchestrator.code_id()?,
                        msg: to_json_binary(&gauge_orchestrator::msg::InstantiateMsg {
                            voting_powers: self.cw4_vote.address()?.to_string(),
                            hook_caller: cw4_addr.to_string(),
                            owner: self.dao_core.address()?.to_string(),
                            gauges: gauge_config.into(),
                        })?,
                        admin: Some(Admin::Address {
                            addr: self.dao_core.address()?.to_string(),
                        }),
                        label: "CW4 Voting Contract".to_owned(),
                        funds: vec![],
                    }],
                    to_disable: vec![],
                },
                &vec![],
                &dao_addr,
            )?
            .event_attr_value("wasm", "prop_module")?;

        Ok(Addr::unchecked(gauge))
    }

    /// instantiate an adapter contract and return its configuration, including the contract addr.
    pub fn init_adapter_return_config(&self, options: &[&str]) -> anyhow::Result<GaugeConfig> {
        let adapter = self.init_adapter(options)?;
        Ok(adapter)
    }
    /// adds an adapter to the existing gauge orchestrator
    pub fn add_adapter_to_gauge(&self, adapter: GaugeConfig) -> anyhow::Result<()> {
        let dao_addr = self.dao_core.address()?;
        self.gauge_suite
            .orchestrator
            .call_as(&dao_addr)
            .create_gauge(adapter)?;
        Ok(())
    }
}

mod gauge {
    use crate::tests::gauges::helpers::default_config;

    use super::*;
    #[test]
    fn test_create_gauge() -> anyhow::Result<(), CwOrchError> {
        let mock = MockBech32::new(PREFIX);
        let mut dao = DaoDaoCw4Gauge::new(mock.clone());
        dao.upload_with_cw4(mock.clone())?;
        dao.default_gauge_setup(mock.clone())?;

        // confirm gauge is created
        let res = dao.gauge_suite.orchestrator.gauge(0)?;
        let adapter = dao.gauge_suite.adapter.addr_str()?;
        assert_eq!(
            res,
            GaugeResponse {
                id: 0,
                title: "default-gauge".to_owned(),
                adapter: adapter,
                epoch_size: EPOCH,
                min_percent_selected: Some(Decimal::percent(5)),
                max_options_selected: 10,
                max_available_percentage: None,
                is_stopped: false,
                next_epoch: mock.block_info()?.time.seconds() + 7 * 86400,
                reset: None,
            }
        );

        Ok(())
    }

    #[test]
    fn test_gauge_can_upgrade_from_self() -> anyhow::Result<()> {
        let mock = MockBech32::new(PREFIX);
        let mut dao = DaoDaoCw4Gauge::new(mock.clone());
        dao.upload_with_cw4(mock.clone())?;
        dao.default_gauge_setup(mock.clone())?;

        // now let's migrate the gauge and make sure nothing breaks
        let dao_addr = dao.dao_core.address()?;
        let gauge_code_id = dao.gauge_suite.orchestrator.code_id()?;
        dao.gauge_suite
            .orchestrator
            .call_as(&dao_addr)
            .migrate(&Empty {}, gauge_code_id)?;
        // confirm contract still functions
        let res = dao.gauge_suite.orchestrator.gauge(0)?;
        let adapter = dao.gauge_suite.adapter.addr_str()?;
        assert_eq!(
            res,
            GaugeResponse {
                id: 0,
                title: "default-gauge".to_owned(),
                adapter: adapter,
                epoch_size: 60 * 60 * 24 * 7,
                min_percent_selected: Some(Decimal::percent(5)),
                max_options_selected: 10,
                max_available_percentage: None,
                is_stopped: false,
                next_epoch: mock.block_info()?.time.seconds() + 7 * 86400,
                reset: None,
            }
        );
        Ok(())
    }
    #[test]
    fn test_gauge_migrate_with_next_epochs() -> anyhow::Result<()> {
        let mock = MockBech32::new(PREFIX);
        let mut dao = DaoDaoCw4Gauge::new(mock.clone());
        dao.upload_with_cw4(mock.clone())?;
        dao.default_gauge_setup(mock.clone())?;
        let gauge_code_id = dao.gauge_suite.orchestrator.code_id()?;
        let dao_addr = dao.dao_core.address()?;
        let gauge_addr = dao.gauge_suite.orchestrator.address()?;
        // now let's migrate the gauge and make sure nothing breaks
        let gauge_id = 0;
        // change next epoch from 7 to 14 days
        mock.call_as(&dao_addr).migrate(
            &MigrateMsg {
                gauge_config: Some(vec![(
                    gauge_id,
                    GaugeMigrationConfig {
                        next_epoch: Some(mock.block_info()?.time.seconds() + 14 * 86400),
                        reset: None,
                    },
                )]),
            },
            gauge_code_id.clone(),
            &gauge_addr.clone(),
        )?;
        // confirm update
        let response = dao.gauge_suite.orchestrator.gauge(0).unwrap();
        assert_eq!(
            response,
            GaugeResponse {
                id: 0,
                title: "default-gauge".to_owned(),
                adapter: dao.gauge_suite.adapter.addr_str()?,
                epoch_size: EPOCH,
                min_percent_selected: Some(Decimal::percent(5)),
                max_options_selected: 10,
                max_available_percentage: None,
                is_stopped: false,
                next_epoch: mock.block_info()?.time.seconds() + 14 * 86400,
                reset: None,
            }
        );
        // try to migrate updating next epoch on nonexisting gauge_id
        mock.migrate(
            &MigrateMsg {
                gauge_config: Some(vec![(
                    420,
                    GaugeMigrationConfig {
                        next_epoch: Some(mock.block_info()?.time.seconds() + 14 * 86400),
                        reset: None,
                    },
                )]),
            },
            gauge_code_id.clone(),
            &gauge_addr.clone(),
        )
        .unwrap_err();
        Ok(())
    }

    // /// attach adaptor in instantiate
    #[test]
    fn test_execute_gauge() -> anyhow::Result<()> {
        let mock = MockBech32::new(PREFIX);
        let mut dao = DaoDaoCw4Gauge::new(mock.clone());
        dao.upload_with_cw4(mock.clone())?;
        dao.default_gauge_setup(mock.clone())?;

        // addresses
        let voter1 = mock.addr_make("voter1");
        let voter2 = mock.addr_make_with_balance("voter2", coins(1000, "ujuno"))?;
        let cw4_vote = dao.cw4_vote.address()?;
        let dao_addr = dao.dao_core.address()?;

        let gauge_id = 0u64;

        let res = dao
            .gauge_suite
            .orchestrator
            .list_options(gauge_id, None, None)?;
        println!("{:#?}", res.options);

        // vote for one of the options in gauge
        dao.gauge_suite.orchestrator.call_as(&voter1).place_votes(
            gauge_id,
            Some(
                vec![GaugeVote {
                    option: voter1.to_string(),
                    weight: Decimal::one(),
                }]
                .into(),
            ),
        )?;
        dao.gauge_suite.orchestrator.call_as(&voter2).place_votes(
            gauge_id,
            Some(
                vec![GaugeVote {
                    option: voter1.to_string(),
                    weight: Decimal::one(),
                }]
                .into(),
            ),
        )?;
        // confirm gauge recieved vote
        let selected_set = dao.gauge_suite.orchestrator.selected_set(gauge_id)?;
        assert_eq!(
            selected_set.votes,
            vec![(voter1.to_string(), Uint128::new(200))]
        );
        // before advancing specified epoch tally won't get sampled
        mock.wait_seconds(EPOCH)?;

        mock.call_as(&dao_addr).execute(
            &GaugeExecuteMsg::Execute { gauge: gauge_id },
            &vec![],
            &dao.gauge_suite.orchestrator.address()?,
        )?;
        // assert rewards have been distriubuted
        assert_eq!(
            mock.balance(voter1, Some("ujuno".into()))?[0].amount,
            Uint128::from(1000u128),
        );

        Ok(())
    }

    #[test]
    fn test_query_last_execution() -> anyhow::Result<()> {
        let mock = MockBech32::new(PREFIX);
        let mut dao = DaoDaoCw4Gauge::new(mock.clone());
        dao.upload_with_cw4(mock.clone())?;
        dao.default_gauge_setup(mock.clone())?;

        // addresses
        let voter1 = mock.addr_make("voter1");
        let voter2 = mock.addr_make_with_balance("voter2", coins(1000, "ujuno"))?;
        let cw4_vote = dao.cw4_vote.address()?;
        let dao_addr = dao.dao_core.address()?;
        let gauge_id = 0;
        let adapter_addr = dao.gauge_suite.adapter.address()?;
        mock.add_balance(&dao_addr, coins(1000, "ujuno"))?;

        // confirm gauge is not executed yet
        assert_eq!(
            dao.gauge_suite
                .orchestrator
                .last_executed_set(gauge_id)?
                .votes,
            None,
        );

        assert_eq!(
            dao.gauge_suite
                .orchestrator
                .last_executed_set(gauge_id)?
                .votes,
            None,
            "not executed yet"
        );
        // vote
        dao.gauge_suite.orchestrator.call_as(&voter1).place_votes(
            gauge_id,
            Some(vec![GaugeVote {
                option: voter1.to_string(),
                weight: Decimal::one(),
            }]),
        )?;
        dao.gauge_suite.orchestrator.call_as(&voter2).place_votes(
            gauge_id,
            Some(vec![
                GaugeVote {
                    option: dao_addr.to_string(),
                    weight: Decimal::percent(40),
                },
                GaugeVote {
                    option: voter2.to_string(),
                    weight: Decimal::percent(60),
                },
            ]),
        )?;

        // wait until epoch passes
        mock.wait_seconds(EPOCH)?;

        // run gauge once
        mock.call_as(&dao_addr).execute(
            &GaugeExecuteMsg::Execute { gauge: gauge_id },
            &vec![],
            &dao.gauge_suite.orchestrator.address()?,
        )?;

        // should return the executed set now
        let expected_votes = Some(vec![
            (voter1.to_string(), Uint128::from(100u128)),
            (voter2.to_string(), Uint128::from(60u128)),
            (dao_addr.to_string(), Uint128::from(40u128)),
        ]);

        assert_eq!(
            dao.gauge_suite
                .orchestrator
                .last_executed_set(gauge_id)?
                .votes,
            expected_votes,
        );

        // change votes
        dao.gauge_suite.orchestrator.call_as(&voter1).place_votes(
            gauge_id,
            Some(
                vec![GaugeVote {
                    option: voter2.to_string(),
                    weight: Decimal::one(),
                }]
                .into(),
            ),
        )?;
        // change votes
        dao.gauge_suite
            .orchestrator
            .call_as(&voter2)
            .place_votes(gauge_id, Some(vec![].into()))?;
        // wait until epoch passes

        mock.wait_seconds(EPOCH)?;
        // should not change last execution yet
        assert_eq!(
            dao.gauge_suite
                .orchestrator
                .last_executed_set(gauge_id)?
                .votes,
            expected_votes,
        );
        // execute
        mock.call_as(&dao_addr).execute(
            &GaugeExecuteMsg::Execute { gauge: gauge_id },
            &vec![],
            &dao.gauge_suite.orchestrator.address()?,
        )?;

        // now it should be changed
        assert_eq!(
            dao.gauge_suite
                .orchestrator
                .last_executed_set(gauge_id)?
                .votes,
            Some(vec![(voter2.to_string(), Uint128::from(100u128))])
        );

        Ok(())
    }

    #[test]
    fn test_execute_gauge_twice_same_epoch() -> anyhow::Result<()> {
        let mock = MockBech32::new(PREFIX);
        let mut dao = DaoDaoCw4Gauge::new(mock.clone());
        dao.upload_with_cw4(mock.clone())?;
        dao.default_gauge_setup(mock.clone())?;

        // addresses
        let voter1 = mock.addr_make("voter1");
        let voter2 = mock.addr_make_with_balance("voter2", coins(1000, "ujuno"))?;
        let cw4_vote = dao.cw4_vote.address()?;
        let dao_addr = dao.dao_core.address()?;
        let gauge_id = 0;
        let adapter_addr = dao.gauge_suite.adapter.address()?;
        mock.add_balance(&dao_addr, coins(1000, "ujuno"))?;

        // vote for one of the options in the gauge
        dao.gauge_suite.orchestrator.call_as(&voter1).place_votes(
            gauge_id,
            Some(
                vec![GaugeVote {
                    option: voter1.to_string(),
                    weight: Decimal::one(),
                }]
                .into(),
            ),
        )?;
        dao.gauge_suite.orchestrator.call_as(&voter2).place_votes(
            gauge_id,
            Some(
                vec![GaugeVote {
                    option: voter1.to_string(),
                    weight: Decimal::one(),
                }]
                .into(),
            ),
        )?;

        // voter1 was option voted for with two 100 voting powers combined
        assert_eq!(
            dao.gauge_suite.orchestrator.selected_set(gauge_id)?.votes,
            vec![(voter1.to_string(), Uint128::new(200u128))]
        );

        // before advancing specified epoch tally won't get sampled
        mock.wait_seconds(EPOCH)?;
        mock.call_as(&dao_addr).execute(
            &GaugeExecuteMsg::Execute { gauge: 0 },
            &vec![],
            &dao.gauge_suite.orchestrator.address()?,
        )?;

        assert_eq!(
            mock.balance(voter1.clone(), Some("ujuno".to_string()))?[0]
                .amount
                .u128(),
            1000u128,
        );
        // execution twice same time won't work
        let err = mock
            .call_as(&dao_addr)
            .execute(
                &GaugeExecuteMsg::Execute { gauge: 0 },
                &vec![],
                &dao.gauge_suite.orchestrator.address()?,
            )
            .unwrap_err();

        let next_epoch = mock.block_info()?.time.seconds() + EPOCH;
        assert_eq!(
            ContractError::EpochNotReached {
                gauge_id: 0,
                current_epoch: mock.block_info()?.time.seconds(),
                next_epoch
            },
            err.downcast().unwrap()
        );
        // just before next epoch fails as well
        mock.wait_seconds(EPOCH - 1)?;
        // execution twice same time won't work
        let err = mock
            .call_as(&dao_addr)
            .execute(
                &GaugeExecuteMsg::Execute { gauge: 0 },
                &vec![],
                &dao.gauge_suite.orchestrator.address()?,
            )
            .unwrap_err();

        assert_eq!(
            ContractError::EpochNotReached {
                gauge_id: 0,
                current_epoch: mock.block_info()?.time.seconds(),
                next_epoch
            },
            err.downcast().unwrap()
        );
        // another epoch is fine
        mock.wait_seconds(EPOCH)?;

        mock.call_as(&dao_addr).execute(
            &GaugeExecuteMsg::Execute { gauge: 0 },
            &vec![],
            &dao.gauge_suite.orchestrator.address()?,
        )?;
        // confirm balance
        assert_eq!(
            mock.balance(voter1.to_string(), Some("ujuno".to_string()))?[0]
                .amount
                .u128(),
            2000u128
        );
        Ok(())
    }

    #[test]
    fn test_execute_stopped_gauge() -> anyhow::Result<()> {
        let mock = MockBech32::new(PREFIX);
        let mut dao = DaoDaoCw4Gauge::new(mock.clone());
        dao.upload_with_cw4(mock.clone())?;
        dao.default_gauge_setup(mock.clone())?;

        // addresses
        let voter1 = mock.addr_make("voter1");
        let voter2 = mock.addr_make_with_balance("voter2", coins(1000, "ujuno"))?;
        let dao_addr = dao.dao_core.address()?;
        let gauge_id = 0;
        mock.add_balance(&dao_addr, coins(1000, "ujuno"))?;

        let not_owner = mock.addr_make("not-owner");

        // stop the gauge by not-owner
        let err = dao
            .gauge_suite
            .orchestrator
            .call_as(&not_owner)
            .stop_gauge(0)
            .unwrap_err();
        assert_eq!(ContractError::Unauthorized {}, err.downcast().unwrap());
        dao.gauge_suite
            .orchestrator
            .call_as(&dao_addr)
            .stop_gauge(0)
            .unwrap();

        // vote for one of the options in the gauge
        dao.gauge_suite.orchestrator.call_as(&voter1).place_votes(
            gauge_id,
            Some(
                vec![GaugeVote {
                    option: voter1.to_string(),
                    weight: Decimal::one(),
                }]
                .into(),
            ),
        )?;
        dao.gauge_suite.orchestrator.call_as(&voter2).place_votes(
            gauge_id,
            Some(
                vec![GaugeVote {
                    option: voter1.to_string(),
                    weight: Decimal::one(),
                }]
                .into(),
            ),
        )?;
        // Despite gauge being stopped, votes have been recorded
        assert_eq!(
            dao.gauge_suite.orchestrator.selected_set(gauge_id)?.votes,
            vec![(voter1.to_string(), Uint128::new(200u128))]
        );

        // before advancing specified epoch tally won't get sampled
        mock.wait_seconds(EPOCH)?;
        let err = mock
            .call_as(&dao_addr)
            .execute(
                &GaugeExecuteMsg::Execute { gauge: 0 },
                &vec![],
                &dao.gauge_suite.orchestrator.address()?,
            )
            .unwrap_err();
        assert_eq!(ContractError::GaugeStopped(0), err.downcast().unwrap());
        Ok(())
    }

    #[test]
    fn test_update_gauge() -> anyhow::Result<()> {
        let mock = MockBech32::new(PREFIX);
        let mut dao = DaoDaoCw4Gauge::new(mock.clone());
        dao.upload_with_cw4(mock.clone())?;
        dao.default_gauge_setup(mock.clone())?;

        // addresses
        let dao_addr = dao.dao_core.address()?;
        let gauge_id = 0;
        let adapter_addr = dao.gauge_suite.adapter.address()?;
        mock.add_balance(&dao_addr, coins(1000, "ujuno"))?;

        // setup another gauge
        let second_gauge_adapter = dao.init_adapter(&[
            &mock.addr_make("voter1").to_string(),
            &mock.addr_make("voter2").to_string(),
        ])?;
        dao.add_adapter_to_gauge(second_gauge_adapter.clone())?;

        let res = dao.gauge_suite.orchestrator.list_gauges(None, None)?;
        assert_eq!(
            res.gauges,
            vec![
                GaugeResponse {
                    id: 0,
                    title: "default-gauge".to_owned(),
                    adapter: dao.gauge_suite.adapter.addr_str()?,
                    epoch_size: EPOCH,
                    min_percent_selected: Some(Decimal::percent(5)),
                    max_options_selected: 10,
                    max_available_percentage: None,
                    is_stopped: false,
                    next_epoch: mock.block_info()?.time.seconds() + 7 * 86400,
                    reset: None,
                },
                GaugeResponse {
                    id: 1,
                    title: "default-gauge".to_owned(),
                    adapter: second_gauge_adapter.adapter.to_string(),
                    epoch_size: EPOCH,
                    min_percent_selected: Some(Decimal::percent(5)),
                    max_options_selected: 10,
                    max_available_percentage: None,
                    is_stopped: false,
                    next_epoch: mock.block_info()?.time.seconds() + 7 * 86400,
                    reset: None,
                }
            ]
        );

        // update parameters on the first gauge
        let fake_owner = mock.addr_make("not-owner");
        let new_epoch = EPOCH * 2;
        let epoch_limit = 8u64;
        let new_min_percent = Some(Decimal::percent(10));
        let new_max_options = 15;
        let new_max_available_percentage = Some(Decimal::percent(5));
        dao.gauge_suite
            .orchestrator
            .call_as(&dao_addr)
            .update_gauge(
                0,
                Some(new_epoch),
                Some(epoch_limit),
                new_max_available_percentage,
                Some(new_max_options),
                new_min_percent,
            )?;

        let res = dao.gauge_suite.orchestrator.list_gauges(None, None)?;
        assert_eq!(
            res.gauges,
            vec![
                GaugeResponse {
                    id: 0,
                    title: "default-gauge".to_owned(),
                    adapter: dao.gauge_suite.adapter.addr_str()?,
                    epoch_size: new_epoch,
                    min_percent_selected: new_min_percent,
                    max_options_selected: new_max_options,
                    max_available_percentage: new_max_available_percentage,
                    is_stopped: false,
                    next_epoch: mock.block_info()?.time.seconds() + 7 * 86400,
                    reset: None,
                },
                GaugeResponse {
                    id: 1,
                    title: "default-gauge".to_owned(),
                    adapter: second_gauge_adapter.adapter.to_string(),
                    epoch_size: EPOCH,
                    min_percent_selected: Some(Decimal::percent(5)),
                    max_options_selected: 10,
                    max_available_percentage: None,
                    is_stopped: false,
                    next_epoch: mock.block_info()?.time.seconds() + 7 * 86400,
                    reset: None,
                }
            ]
        );

        // clean setting of min_percent_selected on second gauge
        dao.gauge_suite
            .orchestrator
            .call_as(&dao_addr)
            .update_gauge(1, None, None, None, None, Some(Decimal::zero()))?;

        let res = dao.gauge_suite.orchestrator.list_gauges(None, None)?;
        assert_eq!(
            res.gauges[1],
            GaugeResponse {
                id: 1,
                title: "default-gauge".to_owned(),
                adapter: second_gauge_adapter.adapter.to_string(),
                epoch_size: EPOCH,
                min_percent_selected: None,
                max_options_selected: 10,
                max_available_percentage: None,
                is_stopped: false,
                next_epoch: mock.block_info()?.time.seconds() + 7 * 86400,
                reset: None,
            }
        );

        // Not owner cannot update gauges
        let err = dao
            .gauge_suite
            .orchestrator
            .call_as(&fake_owner)
            .update_gauge(0, None, None, None, None, Some(Decimal::zero()))
            .unwrap_err();
        assert_eq!(ContractError::Unauthorized {}, err.downcast().unwrap());

        let err = dao
            .gauge_suite
            .orchestrator
            .call_as(&dao_addr)
            .update_gauge(0, Some(50), None, None, None, None)
            .unwrap_err();
        assert_eq!(ContractError::EpochSizeTooShort {}, err.downcast().unwrap());

        let err = dao
            .gauge_suite
            .orchestrator
            .call_as(&dao_addr)
            .update_gauge(
                0,
                None,
                None,
                None,
                Some(new_max_options),
                Some(Decimal::one()),
            )
            .unwrap_err();
        assert_eq!(
            ContractError::MinPercentSelectedTooBig {},
            err.downcast().unwrap()
        );
        let err = dao
            .gauge_suite
            .orchestrator
            .call_as(&dao_addr)
            .update_gauge(0, None, None, None, Some(0), None)
            .unwrap_err();
        assert_eq!(
            ContractError::MaxOptionsSelectedTooSmall {},
            err.downcast().unwrap()
        );
        let err = dao
            .gauge_suite
            .orchestrator
            .call_as(&dao_addr)
            .update_gauge(0, None, None, Some(Decimal::percent(101)), None, None)
            .unwrap_err();
        assert_eq!(
            ContractError::MaxAvailablePercentTooBig {},
            err.downcast().unwrap()
        );
        Ok(())
    }
}

mod reset {
    use super::*;
    use crate::tests::gauges::helpers::RESET_EPOCH;
    use gauge_orchestrator::{msg::ResetMigrationConfig, state::Reset};

    #[test]
    fn test_basic_gauge_reset() -> anyhow::Result<()> {
        let mock = MockBech32::new(PREFIX);
        let mut dao = DaoDaoCw4Gauge::new(mock.clone());
        let voter1 = mock.addr_make("voter1");
        let voter2 = mock.addr_make("voter2");

        dao.upload_with_cw4(mock.clone())?;
        dao.default_gauge_setup(mock.clone())?;
        let dao_addr = dao.dao_core.address()?;
        // setup second gauge adapter with reset configuration
        let mut second_gauge =
            dao.init_adapter_return_config(&[voter1.as_str(), voter2.as_str()])?;
        second_gauge.reset_epoch = Some(RESET_EPOCH);
        dao.add_adapter_to_gauge(second_gauge)?;

        let gauge_id = 1;

        // vote for one of the options in gauge
        dao.gauge_suite.orchestrator.call_as(&voter1).place_votes(
            gauge_id,
            Some(
                vec![GaugeVote {
                    option: voter1.to_string(),
                    weight: Decimal::one(),
                }]
                .into(),
            ),
        )?;
        dao.gauge_suite.orchestrator.call_as(&voter2).place_votes(
            gauge_id,
            Some(
                vec![GaugeVote {
                    option: voter1.to_string(),
                    weight: Decimal::one(),
                }]
                .into(),
            ),
        )?;
        // voter1 was option voted for with two 100 voting powers combined
        assert_eq!(
            dao.gauge_suite.orchestrator.selected_set(gauge_id)?.votes,
            vec![(voter1.to_string(), Uint128::new(200u128))]
        );

        // cannot reset before epoch has passed
        assert_eq!(
            ContractError::ResetEpochNotPassed {},
            dao.gauge_suite
                .orchestrator
                .call_as(&voter2)
                .reset_gauge(10, gauge_id)
                .unwrap_err()
                .downcast()
                .unwrap()
        );

        // reset
        mock.wait_seconds(RESET_EPOCH)?;

        dao.gauge_suite.orchestrator.reset_gauge(100, gauge_id)?;
        // check that gauge was reset
        let selected = dao.gauge_suite.orchestrator.selected_set(gauge_id)?;
        assert_eq!(selected.votes, vec![]);
        assert_eq!(
            dao.gauge_suite
                .orchestrator
                .vote(gauge_id, voter1.clone())?
                .vote,
            None
        );
        assert_eq!(
            dao.gauge_suite
                .orchestrator
                .vote(gauge_id, voter2.clone())?
                .vote,
            None
        );
        // options should still be there
        assert_eq!(
            dao.gauge_suite
                .orchestrator
                .list_options(gauge_id, None, None)
                .unwrap()
                .options,
            vec![
                (voter2.to_string(), Uint128::new(0u128)),
                (voter1.to_string(), Uint128::new(0u128))
            ]
        );

        // actually execute
        mock.call_as(&dao_addr).execute(
            &GaugeExecuteMsg::Execute { gauge: gauge_id },
            &vec![],
            &dao.gauge_suite.orchestrator.address()?,
        )?;

        assert_eq!(
            mock.balance(dao_addr.clone(), Some("ujuno".into()))?[0].amount,
            Uint128::from(10000u128)
        );
        // vote again
        dao.gauge_suite.orchestrator.call_as(&voter1).place_votes(
            gauge_id,
            Some(
                vec![GaugeVote {
                    option: voter2.to_string(),
                    weight: Decimal::one(),
                }]
                .into(),
            ),
        )?;

        // check that vote counts
        let selected = dao.gauge_suite.orchestrator.selected_set(gauge_id)?;
        assert_eq!(
            selected.votes,
            vec![(voter2.to_string(), Uint128::new(100u128))]
        );
        // another epoch is fine
        mock.wait_seconds(EPOCH)?;
        mock.call_as(&dao_addr).execute(
            &GaugeExecuteMsg::Execute { gauge: gauge_id },
            &vec![],
            &dao.gauge_suite.orchestrator.address()?,
        )?;
        assert_eq!(
            mock.balance(&voter2, Some("ujuno".into()))?[0].amount,
            Uint128::from(1000u128)
        );

        Ok(())
    }
    #[test]
    fn test_gauge_migrate_with_reset() -> anyhow::Result<()> {
        let mock = MockBech32::new(PREFIX);
        let mut dao = DaoDaoCw4Gauge::new(mock.clone());
        let voter1 = mock.addr_make("voter1");
        let voter2 = mock.addr_make("voter2");

        dao.upload_with_cw4(mock.clone())?;
        dao.default_gauge_setup(mock.clone())?;
        let dao_addr = dao.dao_core.address()?;
        let gauge_addr = dao.gauge_suite.orchestrator.address()?;
        // now let's migrate the gauge and make sure nothing breaks
        mock.call_as(&dao_addr)
            .migrate(
                &MigrateMsg {
                    gauge_config: Some(vec![(
                        0,
                        GaugeMigrationConfig {
                            reset: Some(ResetMigrationConfig {
                                reset_epoch: RESET_EPOCH,
                                next_reset: mock.block_info()?.time.seconds() - 1,
                            }),
                            next_epoch: None,
                        },
                    )]),
                },
                0,
                &gauge_addr.clone(),
            )
            .unwrap_err();
        // migrate to reset epoch
        mock.call_as(&dao_addr).migrate(
            &MigrateMsg {
                gauge_config: Some(vec![(
                    0,
                    GaugeMigrationConfig {
                        reset: Some(ResetMigrationConfig {
                            reset_epoch: RESET_EPOCH,
                            next_reset: mock.block_info()?.time.seconds() + 100,
                        }),
                        next_epoch: None,
                    },
                )]),
            },
            dao.gauge_suite.orchestrator.code_id()?,
            &gauge_addr.clone(),
        )?;

        // check that gauge was migrated
        let res = dao.gauge_suite.orchestrator.gauge(0)?;
        assert_eq!(
            res,
            GaugeResponse {
                id: 0,
                title: "default-gauge".to_owned(),
                adapter: dao.gauge_suite.adapter.addr_str()?,
                epoch_size: EPOCH,
                min_percent_selected: Some(Decimal::percent(5)),
                max_options_selected: 10,
                max_available_percentage: None,
                is_stopped: false,
                next_epoch: mock.block_info()?.time.seconds() + 7 * 86400,
                reset: Some(Reset {
                    last: None,
                    reset_each: RESET_EPOCH,
                    next: mock.block_info()?.time.seconds() + 100,
                    total: None,
                })
            }
        );

        Ok(())
    }
    #[test]
    fn test_gauge_migrate_keeps_last_reset() -> anyhow::Result<()> {
        let mock = MockBech32::new(PREFIX);
        let mut dao = DaoDaoCw4Gauge::new(mock.clone());
        dao.upload_with_cw4(mock.clone())?;
        dao.default_gauge_setup(mock.clone())?;

        let voter1 = mock.addr_make("voter1");
        let voter2 = mock.addr_make("voter2");
        let gauge_id = 1;
        let dao_addr = dao.dao_core.address()?;
        let gauge_addr = dao.gauge_suite.orchestrator.address()?;

        // setup second gauge adapter with reset configuration
        let mut second_gauge =
            dao.init_adapter_return_config(&[voter1.as_str(), voter2.as_str()])?;
        second_gauge.reset_epoch = Some(RESET_EPOCH);
        dao.add_adapter_to_gauge(second_gauge)?;

        // reset once before migration
        mock.wait_seconds(RESET_EPOCH)?;
        dao.gauge_suite
            .orchestrator
            .call_as(&dao_addr)
            .reset_gauge(1, gauge_id)?;

        let gauge = dao.gauge_suite.orchestrator.gauge(gauge_id)?;
        assert_eq!(
            gauge.reset.unwrap().last,
            Some(mock.block_info()?.time.seconds())
        );

        // now let's migrate the gauge and make sure nothing breaks
        // migrate to reset epoch
        mock.call_as(&dao_addr).migrate(
            &MigrateMsg {
                gauge_config: Some(vec![(
                    0,
                    GaugeMigrationConfig {
                        reset: Some(ResetMigrationConfig {
                            reset_epoch: RESET_EPOCH,
                            next_reset: mock.block_info()?.time.seconds() + 100,
                        }),
                        next_epoch: None,
                    },
                )]),
            },
            dao.gauge_suite.orchestrator.code_id()?,
            &gauge_addr.clone(),
        )?;

        // migrate
        Ok(())
    }
    #[test]
    fn test_partial_reset() -> anyhow::Result<()> {
        let mock = MockBech32::new(PREFIX);
        let mut dao = DaoDaoCw4Gauge::new(mock.clone());
        let voter1 = mock.addr_make("voter1");
        let voter2 = mock.addr_make("voter2");
        dao.upload_with_cw4(mock.clone())?;
        dao.default_gauge_setup(mock.clone())?;

        // setup second gauge adapter with reset configuration
        let mut second_gauge =
            dao.init_adapter_return_config(&[voter1.as_str(), voter2.as_str()])?;
        second_gauge.reset_epoch = Some(RESET_EPOCH);
        dao.add_adapter_to_gauge(second_gauge)?;

        // addresses
        let dao_addr = dao.dao_core.address()?;
        let adapter_addr = dao.gauge_suite.adapter.address()?;
        mock.add_balance(&dao_addr, coins(1000, "ujuno"))?;
        let gauge_id = 1;

        // vote for the gauge options
        // vote again
        dao.gauge_suite.orchestrator.call_as(&voter1).place_votes(
            gauge_id,
            Some(
                vec![GaugeVote {
                    option: voter1.to_string(),
                    weight: Decimal::one(),
                }]
                .into(),
            ),
        )?;
        dao.gauge_suite.orchestrator.call_as(&voter2).place_votes(
            gauge_id,
            Some(
                vec![GaugeVote {
                    option: voter2.to_string(),
                    weight: Decimal::one(),
                }]
                .into(),
            ),
        )?;
        // start resetting
        mock.wait_seconds(RESET_EPOCH)?;
        mock.call_as(&dao_addr).execute(
            &GaugeExecuteMsg::ResetGauge {
                gauge: gauge_id,
                batch_size: 1,
            },
            &vec![],
            &dao.gauge_suite.orchestrator.address()?,
        )?;
        // try to vote during reset
        assert_eq!(
            ContractError::GaugeResetting(gauge_id),
            dao.gauge_suite
                .orchestrator
                .call_as(&voter2)
                .place_votes(
                    gauge_id,
                    Some(
                        vec![GaugeVote {
                            option: voter2.to_string(),
                            weight: Decimal::one(),
                        }]
                        .into(),
                    ),
                )
                .unwrap_err()
                .downcast()
                .unwrap()
        );

        // check selected set query
        let selected = dao.gauge_suite.orchestrator.selected_set(gauge_id)?;
        assert_eq!(selected.votes, vec![]);
        // check votes list
        let votes = dao
            .gauge_suite
            .orchestrator
            .list_votes(gauge_id, None, None)?;
        assert_eq!(votes.votes, vec![]);

        // finish resetting
        dao.gauge_suite.orchestrator.reset_gauge(1, gauge_id)?;

        Ok(())
    }
}

mod tally {
    use cw4::{MemberChangedHookMsg, MemberDiff};

    use super::*;
    //     use dao_voting_cw4::msg::QueryMsgFns;

    fn defualt_voters(mock: MockBech32, number: Vec<u64>) -> anyhow::Result<Vec<MemberDiff>> {
        let mut voters = vec![];

        for n in 0..number.len() {
            let weight = n as u64;
            let voter = mock
                .addr_make_with_balance(format!("voter{}", n + 2).as_str(), coins(1000, "ujuno"))?;
            voters.push(MemberDiff {
                key: voter.to_string(),
                old: None,
                new: Some(weight),
            })
        }
        Ok(voters)
    }
    fn members_to_addr(mock: MockBech32, members: Vec<MemberDiff>) -> anyhow::Result<Vec<Addr>> {
        let mut addrs = vec![];
        for member in members {
            let addr = mock.addr_make(&member.key);
            addrs.push(addr)
        }
        Ok(addrs)
    }

    #[test]
    fn test_multiple_options_one_gauge() -> anyhow::Result<()> {
        let mock = MockBech32::new(PREFIX);
        let mut dao = DaoDaoCw4Gauge::new(mock.clone());
        dao.upload_with_cw4(mock.clone())?;
        dao.default_gauge_setup(mock.clone())?;

        let voter1 = mock.addr_make("voter3");
        let voter2 = mock.addr_make("voter4");
        let voter3 = mock.addr_make("voter5");
        let voter4 = mock.addr_make("voter6");
        let voter5 = mock.addr_make("voter7");

        // create new gauge with more members
        let members = defualt_voters(mock.clone(), vec![600, 120, 130, 140, 150])?;
        let voters = members_to_addr(mock.clone(), members.clone())?;
        let cw4 = dao.cw4_vote.address()?;
        let gauge2 = dao.init_adapter(&["option1", "option2", "option3", "option4", "option5"])?;
        dao.add_adapter_to_gauge(gauge2)?;
        let gauge_id = 1;

        mock.call_as(&cw4).execute(
            &gauge_orchestrator::msg::ExecuteMsg::MemberChangedHook(MemberChangedHookMsg {
                diffs: members,
            }),
            &vec![],
            &dao.gauge_suite.orchestrator.address()?,
        )?;
        mock.wait_blocks(1)?;

        dao.gauge_suite.orchestrator.call_as(&voter1).place_votes(
            gauge_id,
            Some(
                vec![GaugeVote {
                    option: "option1".to_string(),
                    weight: Decimal::one(),
                }]
                .into(),
            ),
        )?;
        dao.gauge_suite.orchestrator.call_as(&voter2).place_votes(
            gauge_id,
            Some(
                vec![GaugeVote {
                    option: "option2".to_string(),
                    weight: Decimal::one(),
                }]
                .into(),
            ),
        )?;

        dao.gauge_suite.orchestrator.call_as(&voter3).place_votes(
            gauge_id,
            Some(
                vec![GaugeVote {
                    option: "option3".to_string(),
                    weight: Decimal::one(),
                }]
                .into(),
            ),
        )?;
        dao.gauge_suite.orchestrator.call_as(&voter4).place_votes(
            gauge_id,
            Some(
                vec![GaugeVote {
                    option: "option4".to_string(),
                    weight: Decimal::one(),
                }]
                .into(),
            ),
        )?;
        dao.gauge_suite.orchestrator.call_as(&voter5).place_votes(
            gauge_id,
            Some(
                vec![GaugeVote {
                    option: "option5".to_string(),
                    weight: Decimal::one(),
                }]
                .into(),
            ),
        )?;

        let selected = dao.gauge_suite.orchestrator.selected_set(gauge_id)?;
        assert_eq!(
            selected.votes,
            vec![
                ("option1".to_owned(), Uint128::new(600)),
                ("option5".to_owned(), Uint128::new(150)),
                ("option4".to_owned(), Uint128::new(140)),
                ("option3".to_owned(), Uint128::new(130)),
                ("option2".to_owned(), Uint128::new(120))
            ]
        );

        dao.gauge_suite.orchestrator.call_as(&voter1).place_votes(
            gauge_id,
            Some(
                vec![GaugeVote {
                    option: "option2".to_string(),
                    weight: Decimal::one(),
                }]
                .into(),
            ),
        )?;

        let selected = dao.gauge_suite.orchestrator.selected_set(gauge_id)?;
        assert_eq!(
            selected.votes,
            vec![
                ("option2".to_owned(), Uint128::new(720)),
                ("option5".to_owned(), Uint128::new(150)),
                ("option4".to_owned(), Uint128::new(140)),
                ("option3".to_owned(), Uint128::new(130)),
            ]
        );

        Ok(())
    }
    #[test]
    fn test_multiple_options_two_gauges() -> anyhow::Result<()> {
        let mock = MockBech32::new(PREFIX);
        let mut dao = DaoDaoCw4Gauge::new(mock.clone());
        dao.upload_with_cw4(mock.clone())?;
        dao.default_gauge_setup(mock.clone())?;

        let voter1 = mock.addr_make("voter3");
        let voter2 = mock.addr_make("voter4");
        let voter3 = mock.addr_make("voter5");
        let voter4 = mock.addr_make("voter6");
        let voter5 = mock.addr_make("voter7");

        let gauge2 = dao.init_adapter_return_config(&["option1", "option2"])?;
        dao.add_adapter_to_gauge(gauge2)?;
        mock.wait_blocks(1)?;
        let gauge3 = dao.init_adapter_return_config(&["option3", "option4", "option5"])?;
        dao.add_adapter_to_gauge(gauge3)?;
        mock.wait_blocks(1)?;

        dao.gauge_suite.orchestrator.call_as(&voter1).place_votes(
            1,
            Some(
                vec![GaugeVote {
                    option: "option2".to_string(),
                    weight: Decimal::one(),
                }]
                .into(),
            ),
        )?;
        dao.gauge_suite.orchestrator.call_as(&voter2).place_votes(
            1,
            Some(
                vec![GaugeVote {
                    option: "option2".to_string(),
                    weight: Decimal::one(),
                }]
                .into(),
            ),
        )?;
        dao.gauge_suite.orchestrator.call_as(&voter3).place_votes(
            2,
            Some(
                vec![GaugeVote {
                    option: "option3".to_string(),
                    weight: Decimal::one(),
                }]
                .into(),
            ),
        )?;
        dao.gauge_suite.orchestrator.call_as(&voter4).place_votes(
            2,
            Some(
                vec![GaugeVote {
                    option: "option5".to_string(),
                    weight: Decimal::one(),
                }]
                .into(),
            ),
        )?;
        dao.gauge_suite.orchestrator.call_as(&voter5).place_votes(
            2,
            Some(
                vec![GaugeVote {
                    option: "option5".to_string(),
                    weight: Decimal::one(),
                }]
                .into(),
            ),
        )?;

        let selected = dao.gauge_suite.orchestrator.selected_set(1)?;
        assert_eq!(
            selected.votes,
            vec![("option2".to_owned(), Uint128::new(720)),]
        );
        let selected = dao.gauge_suite.orchestrator.selected_set(2)?;
        assert_eq!(
            selected.votes,
            vec![
                ("option5".to_owned(), Uint128::new(290)),
                ("option3".to_owned(), Uint128::new(130)),
            ]
        );

        Ok(())
    }

    #[test]
    fn test_not_voted_options_are_not_selected() -> anyhow::Result<()> {
        let mock = MockBech32::new(PREFIX);
        let mut dao = DaoDaoCw4Gauge::new(mock.clone());
        dao.upload_with_cw4(mock.clone())?;
        dao.default_gauge_setup(mock.clone())?;

        let voter1 = mock.addr_make("voter3");
        let voter2 = mock.addr_make("voter4");
        let voter3 = mock.addr_make("voter5");
        let voter4 = mock.addr_make("voter6");
        let voter5 = mock.addr_make("voter7");

        let members = defualt_voters(mock.clone(), vec![600, 120])?;
        let voters = members_to_addr(mock.clone(), members.clone())?;

        let gauge =
            dao.init_adapter_return_config(&["option1", "option2", "option3", "option4"])?;
        dao.add_adapter_to_gauge(gauge)?;

        dao.gauge_suite.orchestrator.call_as(&voter1).place_votes(
            1,
            Some(
                vec![GaugeVote {
                    option: "option1".to_string(),
                    weight: Decimal::one(),
                }]
                .into(),
            ),
        )?;
        dao.gauge_suite.orchestrator.call_as(&voter2).place_votes(
            1,
            Some(
                vec![GaugeVote {
                    option: "option2".to_string(),
                    weight: Decimal::one(),
                }]
                .into(),
            ),
        )?;

        let selected = dao.gauge_suite.orchestrator.selected_set(1)?;
        assert_eq!(
            selected.votes,
            vec![
                ("option1".to_owned(), Uint128::new(600)),
                ("option2".to_owned(), Uint128::new(120)),
            ]
        );

        // first voter changes vote to option2
        dao.gauge_suite.orchestrator.call_as(&voter1).place_votes(
            1,
            Some(
                vec![GaugeVote {
                    option: "option2".to_string(),
                    weight: Decimal::one(),
                }]
                .into(),
            ),
        )?;

        let selected = dao.gauge_suite.orchestrator.selected_set(1)?;
        assert_eq!(
            selected.votes,
            vec![("option2".to_owned(), Uint128::new(720)),]
        );

        Ok(())
    }
}

mod voting {
    //     use std::vec;

    use cw4::{MemberChangedHookMsg, MemberDiff};
    use cw4_group::msg::ExecuteMsg as Cw4ExecuteMsg;
    use dao_hooks::nft_stake::NftStakeChangedHookMsg;
    use dao_interface::CoreQueryMsgFns;
    use gauge_orchestrator::msg::VoteInfo;

    use crate::tests::gauges::helpers::{default_config, multi_vote, simple_vote};

    use dao_gauge_adapter::contract::ExecuteMsgFns;

    use super::*;

    #[test]
    fn test_add_option() -> anyhow::Result<()> {
        let mock = MockBech32::new(PREFIX);
        let mut dao = DaoDaoCw4Gauge::new(mock.clone());
        dao.upload_with_cw4(mock.clone())?;
        dao.default_gauge_setup(mock.clone())?;

        let dao_addr = dao.dao_core.address()?;
        let voter1 = mock.addr_make_with_balance("voter1", coins(1000, "ujuno"))?;
        let voter2 = mock.addr_make_with_balance("voter2", coins(1000, "ujuno"))?;
        let not_voter = mock.addr_make_with_balance("not-voter", coins(1000, "ujuno"))?;

        // gauge returns list all options; it does query adapter at initialization
        let options = dao.gauge_suite.orchestrator.list_options(0, None, None)?;
        assert_eq!(options.options.len(), 3);

        // add moe valid options to gauge adapter

        dao.gauge_suite
            .test_adapter
            .call_as(&dao_addr)
            .add_valid_option("addedoption1")?;
        dao.gauge_suite
            .test_adapter
            .call_as(&dao_addr)
            .add_valid_option("addedoption2")?;

        // Voting members can add options
        dao.gauge_suite
            .orchestrator
            .call_as(&voter1)
            .add_option(0, "addedoption1")?;
        dao.gauge_suite
            .orchestrator
            .call_as(&voter2)
            .add_option(0, "addedoption2")?;

        // added options are automatically voted for by creators
        let options = dao.gauge_suite.orchestrator.list_options(0, None, None)?;
        assert_eq!(
            options.options,
            vec![
                ("addedoption1".to_owned(), Uint128::zero()),
                ("addedoption2".to_owned(), Uint128::zero()),
                (voter2.to_string(), Uint128::zero()),
                (voter1.to_string(), Uint128::zero()),
                (dao_addr.to_string(), Uint128::zero()),
            ]
        );

        // add another valid option to gauge adapter
        dao.gauge_suite
            .test_adapter
            .call_as(&dao_addr)
            .add_valid_option("addedoption3")?;
        // Non-voting members cannot add options
        let err = dao
            .gauge_suite
            .orchestrator
            .call_as(&not_voter)
            .add_option(0, "addedoption3")
            .unwrap_err();
        assert_eq!(
            ContractError::NoVotingPower(not_voter.to_string()),
            err.downcast().unwrap()
        );

        Ok(())
    }
    #[test]
    fn test_remove_option() -> anyhow::Result<()> {
        let mock = MockBech32::new(PREFIX);
        let mut dao = DaoDaoCw4Gauge::new(mock.clone());
        dao.upload_with_cw4(mock.clone())?;
        dao.default_gauge_setup(mock.clone())?;
        let dao_addr = dao.dao_core.address()?;
        let voter1 = mock.addr_make_with_balance("voter1", coins(1000, "ujuno"))?;
        let voter2 = mock.addr_make_with_balance("voter2", coins(1000, "ujuno"))?;
        let gauge_id = 0;

        // gauge returns list all options; it does query adapter at initialization
        let options = dao
            .gauge_suite
            .orchestrator
            .list_options(gauge_id, None, None)?;
        assert_eq!(options.options.len(), 3);

        // add new valid options to the gauge adapter
        dao.gauge_suite
            .test_adapter
            .call_as(&dao_addr)
            .add_valid_option("addedoption1")?;
        dao.gauge_suite
            .test_adapter
            .call_as(&dao_addr)
            .add_valid_option("addedoption2")?;

        // Voting members can add options
        dao.gauge_suite
            .orchestrator
            .call_as(&voter1)
            .add_option(gauge_id, "addedoption1")?;
        dao.gauge_suite
            .orchestrator
            .call_as(&voter2)
            .add_option(gauge_id, "addedoption2")?;

        let options = dao
            .gauge_suite
            .orchestrator
            .list_options(gauge_id, None, None)?;
        assert_eq!(options.options.len(), 5);

        // owner can remove an option that has been added already
        dao.gauge_suite
            .orchestrator
            .call_as(&dao_addr)
            .remove_option(gauge_id, "addedoption1")?;
        // Anyone else cannot remove options
        let err = dao
            .gauge_suite
            .orchestrator
            .call_as(&voter1)
            .remove_option(gauge_id, "addedoption2")
            .unwrap_err();
        assert_eq!(ContractError::Unauthorized {}, err.downcast().unwrap());
        // one has been removed
        let options = dao
            .gauge_suite
            .orchestrator
            .list_options(gauge_id, None, None)?;
        assert_eq!(options.options.len(), 4);

        // invalidate added option
        mock.call_as(&dao_addr).execute(
            &AdapterExecuteMsg::InvalidateOption {
                option: "addedoption2".into(),
            },
            &vec![],
            &dao.gauge_suite.test_adapter.address()?,
        )?;
        // owner can remove an option that is no longer valid
        dao.gauge_suite
            .orchestrator
            .call_as(&dao_addr)
            .remove_option(gauge_id, "addedoption2")?;
        // Both options are now removed
        let options = dao
            .gauge_suite
            .orchestrator
            .list_options(gauge_id, None, None)?;
        assert_eq!(options.options.len(), 3);

        Ok(())
    }

    #[test]
    fn test_vote_for_option() -> anyhow::Result<()> {
        let mock = MockBech32::new(PREFIX);
        let mut dao = DaoDaoCw4Gauge::new(mock.clone());
        dao.upload_with_cw4(mock.clone())?;
        dao.default_gauge_setup(mock.clone())?;
        let dao_addr = dao.dao_core.address()?;
        let voter1 = mock.addr_make_with_balance("voter1", coins(1000, "ujuno"))?;
        let voter2 = mock.addr_make_with_balance("voter2", coins(1000, "ujuno"))?;
        let gauge_id = 0;
        let adapter = dao.gauge_suite.test_adapter.address()?;

        // vote for option from adapter (voting members are by default
        // options in adapter in this test suite)
        dao.gauge_suite.orchestrator.call_as(&voter1).place_votes(
            gauge_id,
            Some(vec![gauge_orchestrator::state::Vote {
                option: voter1.to_string(),
                weight: Decimal::percent(90),
            }]),
        )?;
        assert_eq!(
            VoteInfo {
                voter: voter1.to_string(),
                votes: vec![gauge_orchestrator::state::Vote {
                    option: voter1.to_string(),
                    weight: Decimal::percent(90),
                }],
                cast: Some(mock.block_info()?.time.seconds()),
            },
            dao.gauge_suite
                .orchestrator
                .vote(gauge_id, voter1.clone())?
                .vote
                .unwrap(),
        );
        // check tally is proper
        let selected = dao.gauge_suite.orchestrator.selected_set(gauge_id)?;
        assert_eq!(
            selected.votes,
            vec![(voter1.to_string(), Uint128::from(90u128))]
        );
        // add new valid options to the gauge adapter
        mock.call_as(&dao_addr).execute(
            &dao_gauge_adapter::contract::ExecuteMsg::AddValidOption {
                option: "option1".into(),
            },
            &vec![],
            &adapter.clone(),
        )?;
        mock.call_as(&dao_addr).execute(
            &dao_gauge_adapter::contract::ExecuteMsg::AddValidOption {
                option: "option2".into(),
            },
            &vec![],
            &adapter.clone(),
        )?;

        // change vote for option added through gauge
        dao.gauge_suite
            .orchestrator
            .call_as(&voter1)
            .add_option(gauge_id, "option1")?;
        dao.gauge_suite
            .orchestrator
            .call_as(&voter2)
            .add_option(gauge_id, "option2")?;
        // voter2 drops vote as well
        dao.gauge_suite.orchestrator.call_as(&voter2).place_votes(
            gauge_id,
            Some(vec![
                gauge_orchestrator::state::Vote {
                    option: "option1".to_string(),
                    weight: Decimal::percent(50),
                },
                gauge_orchestrator::state::Vote {
                    option: "option2".to_string(),
                    weight: Decimal::percent(50),
                },
            ]),
        )?;
        assert_eq!(
            vec![
                multi_vote(
                    &voter2.to_string(),
                    &[("option1", 50), ("option2", 50)],
                    mock.block_info()?.time.seconds(),
                ),
                simple_vote(
                    &voter1.to_string(),
                    &voter1.to_string(),
                    90,
                    mock.block_info()?.time.seconds()
                ),
            ],
            dao.gauge_suite
                .orchestrator
                .list_votes(gauge_id, None, None)?
                .votes
        );

        // placing vote again overwrites previous ones
        dao.gauge_suite.orchestrator.call_as(&voter1).place_votes(
            gauge_id,
            Some(vec![gauge_orchestrator::state::Vote {
                option: "option1".to_string(),
                weight: Decimal::percent(90),
            }]),
        )?;
        dao.gauge_suite.orchestrator.call_as(&voter2).place_votes(
            gauge_id,
            Some(vec![gauge_orchestrator::state::Vote {
                option: "option1".to_string(),
                weight: Decimal::percent(90),
            }]),
        )?;
        assert_eq!(
            vec![
                simple_vote(
                    &voter2.to_string(),
                    "option1",
                    90,
                    mock.block_info()?.time.seconds()
                ),
                simple_vote(
                    &voter1.to_string(),
                    "option1",
                    90,
                    mock.block_info()?.time.seconds()
                ),
            ],
            dao.gauge_suite
                .orchestrator
                .list_votes(gauge_id, None, None)?
                .votes,
        );

        // vote for non-existing option
        let err = dao
            .gauge_suite
            .orchestrator
            .call_as(&voter2)
            .place_votes(
                gauge_id,
                Some(vec![gauge_orchestrator::state::Vote {
                    option: "option420".to_string(),
                    weight: Decimal::percent(90),
                }]),
            )
            .unwrap_err();
        assert_eq!(
            ContractError::OptionDoesNotExists {
                option: "option420".to_owned(),
                gauge_id,
            },
            err.downcast().unwrap()
        );

        Ok(())
    }

    #[test]
    fn test_remove_vote() -> anyhow::Result<()> {
        let mock = MockBech32::new(PREFIX);
        let mut dao = DaoDaoCw4Gauge::new(mock.clone());
        dao.upload_with_cw4(mock.clone())?;
        dao.default_gauge_setup(mock.clone())?;
        let dao_addr = dao.dao_core.address()?;
        let voter1 = mock.addr_make_with_balance("voter1", coins(1000, "ujuno"))?;
        let voter2 = mock.addr_make_with_balance("voter2", coins(1000, "ujuno"))?;
        let gauge_id = 0;
        let adapter = dao.gauge_suite.test_adapter.address()?;

        // vote for option from adapter (voting members are by default
        // options in adapter in this test suite)
        dao.gauge_suite.orchestrator.call_as(&voter1).place_votes(
            gauge_id,
            Some(vec![gauge_orchestrator::state::Vote {
                option: voter1.to_string(),
                weight: Decimal::one(),
            }]),
        )?;
        dao.gauge_suite.orchestrator.call_as(&voter2).place_votes(
            gauge_id,
            Some(vec![gauge_orchestrator::state::Vote {
                option: voter1.to_string(),
                weight: Decimal::one(),
            }]),
        )?;

        assert_eq!(
            vec![
                simple_vote(
                    &voter2.to_string(),
                    &voter1.to_string(),
                    100,
                    mock.block_info()?.time.seconds()
                ),
                simple_vote(
                    &voter1.to_string(),
                    &voter1.to_string(),
                    100,
                    mock.block_info()?.time.seconds()
                ),
            ],
            dao.gauge_suite
                .orchestrator
                .list_votes(gauge_id, None, None)?
                .votes
        );

        // remove vote
        dao.gauge_suite
            .orchestrator
            .call_as(&voter1)
            .place_votes(gauge_id, None)?;
        assert_eq!(
            vec![simple_vote(
                &voter2.to_string(),
                &voter1.to_string(),
                100,
                mock.block_info()?.time.seconds()
            )],
            dao.gauge_suite
                .orchestrator
                .list_votes(gauge_id, None, None)?
                .votes
        );
        assert_eq!(
            None,
            dao.gauge_suite
                .orchestrator
                .vote(gauge_id, voter1.to_string())?
                .vote
        );
        assert_eq!(
            Some(simple_vote(
                &voter2.to_string(),
                &voter1.to_string(),
                100,
                mock.block_info()?.time.seconds()
            )),
            dao.gauge_suite
                .orchestrator
                .vote(gauge_id, voter2.to_string())?
                .vote
        );

        // remove nonexisting vote
        let err = dao
            .gauge_suite
            .orchestrator
            .call_as(&voter1)
            .place_votes(gauge_id, None)
            .unwrap_err();
        assert_eq!(
            ContractError::CannotRemoveNonexistingVote {},
            err.downcast().unwrap()
        );

        Ok(())
    }
    #[test]
    fn test_votes_stays_the_same_after_execution() -> anyhow::Result<()> {
        let mock = MockBech32::new(PREFIX);
        let mut dao = DaoDaoCw4Gauge::new(mock.clone());
        dao.upload_with_cw4(mock.clone())?;
        dao.default_gauge_setup(mock.clone())?;
        let dao_addr = dao.dao_core.address()?;
        let voter1 = mock.addr_make_with_balance("voter1", coins(1000, "ujuno"))?;
        let voter2 = mock.addr_make_with_balance("voter2", coins(1000, "ujuno"))?;
        let gauge_id = 0;
        let adapter = dao.gauge_suite.test_adapter.address()?;

        // vote for one of the options in gauge
        dao.gauge_suite.orchestrator.call_as(&voter1).place_votes(
            gauge_id,
            Some(vec![gauge_orchestrator::state::Vote {
                option: voter1.to_string(),
                weight: Decimal::one(),
            }]),
        )?;
        dao.gauge_suite.orchestrator.call_as(&voter2).place_votes(
            gauge_id,
            Some(vec![gauge_orchestrator::state::Vote {
                option: voter1.to_string(),
                weight: Decimal::one(),
            }]),
        )?;

        // voter1 was option voted for with two 100 voting powers combined
        let selected = dao.gauge_suite.orchestrator.selected_set(gauge_id)?;
        assert_eq!(
            selected.votes,
            vec![(voter1.to_string(), Uint128::new(200))]
        );
        // before advancing specified epoch tally won't get sampled
        mock.wait_seconds(EPOCH)?;
        assert_eq!(
            vec![
                simple_vote(
                    &voter2.to_string(),
                    &voter1.to_string(),
                    100,
                    mock.block_info()?.time.seconds() - EPOCH
                ),
                simple_vote(
                    &voter1.to_string(),
                    &voter1.to_string(),
                    100,
                    mock.block_info()?.time.seconds() - EPOCH
                )
            ],
            dao.gauge_suite
                .orchestrator
                .list_votes(gauge_id, None, None)?
                .votes
        );

        mock.call_as(&dao_addr).execute(
            &GaugeExecuteMsg::Execute { gauge: gauge_id },
            &vec![],
            &dao.gauge_suite.orchestrator.address()?,
        )?;

        assert_eq!(
            vec![
                simple_vote(
                    &voter2.to_string(),
                    &voter1.to_string(),
                    100,
                    mock.block_info()?.time.seconds() - EPOCH
                ),
                simple_vote(
                    &voter1.to_string(),
                    &voter1.to_string(),
                    100,
                    mock.block_info()?.time.seconds() - EPOCH
                ),
            ],
            dao.gauge_suite
                .orchestrator
                .list_votes(gauge_id, None, None)?
                .votes
        );

        assert_eq!(
            Some(simple_vote(
                &voter1.to_string(),
                &voter1.to_string(),
                100,
                mock.block_info()?.time.seconds() - EPOCH
            )),
            dao.gauge_suite
                .orchestrator
                .vote(gauge_id, voter1.to_string())?
                .vote
        );
        assert_eq!(
            Some(simple_vote(
                &voter2.to_string(),
                &voter1.to_string(),
                100,
                mock.block_info()?.time.seconds() - EPOCH
            )),
            dao.gauge_suite
                .orchestrator
                .vote(gauge_id, voter2.to_string())?
                .vote
        );

        Ok(())
    }
    #[test]
    fn test_vote_for_max_capped_option() -> anyhow::Result<()> {
        let mock = MockBech32::new(PREFIX);
        let mut dao = DaoDaoCw4Gauge::new(mock.clone());
        dao.upload_with_cw4(mock.clone())?;
        dao.default_gauge_setup(mock.clone())?;
        let dao_addr = dao.dao_core.address()?;
        let voter1 = mock.addr_make_with_balance("voter1", coins(1000, "ujuno"))?;
        let voter2 = mock.addr_make_with_balance("voter2", coins(1000, "ujuno"))?;

        let mut gauge = dao.init_adapter_return_config(&[voter1.as_str(), voter2.as_str()])?;
        gauge.max_available_percentage = Some(Decimal::percent(10));
        dao.add_adapter_to_gauge(gauge)?;
        let gauge_id = 1;

        // wait until epoch passes
        mock.wait_seconds(EPOCH)?;

        // add more valid options to gauge adapter
        mock.call_as(&dao_addr).execute(
            &dao_gauge_adapter::contract::ExecuteMsg::AddValidOption {
                option: "option1".into(),
            },
            &vec![],
            &dao.gauge_suite.test_adapter.address()?,
        )?;
        mock.call_as(&dao_addr).execute(
            &dao_gauge_adapter::contract::ExecuteMsg::AddValidOption {
                option: "option2".into(),
            },
            &vec![],
            &dao.gauge_suite.test_adapter.address()?,
        )?;

        // change vote for option added through gauge
        dao.gauge_suite
            .orchestrator
            .call_as(&voter1)
            .add_option(gauge_id, "option1")?;
        dao.gauge_suite
            .orchestrator
            .call_as(&voter2)
            .add_option(gauge_id, "option2")?;

        // vote 100% voting power on 'voter1' option (100 weight)
        dao.gauge_suite.orchestrator.call_as(&voter1).place_votes(
            gauge_id,
            Some(vec![gauge_orchestrator::state::Vote {
                option: "option1".to_string(),
                weight: Decimal::one(),
            }]),
        )?;

        // vote 10% voting power on 'voter2' option (10 weight)
        dao.gauge_suite.orchestrator.call_as(&voter2).place_votes(
            gauge_id,
            Some(vec![gauge_orchestrator::state::Vote {
                option: "option2".to_string(),
                weight: Decimal::percent(10),
            }]),
        )?;

        assert_eq!(
            vec![
                multi_vote(
                    &voter2.to_string(),
                    &[("option2", 10)],
                    mock.block_info()?.time.seconds(),
                ),
                multi_vote(
                    &voter1.to_string(),
                    &[("option1", 100)],
                    mock.block_info()?.time.seconds(),
                ),
            ],
            dao.gauge_suite
                .orchestrator
                .list_votes(gauge_id, None, None)?
                .votes
        );

        let selected_set = dao.gauge_suite.orchestrator.selected_set(gauge_id)?.votes;
        // Despite 'option1' having 100 voting power and option2 having 10 voting power,
        // because of max vote cap set to 10% now 'option1' will have its power decreased to 10% * 110
        // 'option2' stays at 10 voting power as it was below 10% of total votes
        assert_eq!(
            selected_set,
            vec![
                ("option1".to_owned(), Uint128::new(11)),
                ("option2".to_owned(), Uint128::new(10))
            ]
        );

        Ok(())
    }
    #[test]
    fn test_membership_voting_power_change() -> anyhow::Result<()> {
        let mock = MockBech32::new(PREFIX);
        let voter1 = mock.addr_make_with_balance("voter1", coins(1000, "ujuno"))?;
        let voter2 = mock.addr_make_with_balance("voter2", coins(1000, "ujuno"))?;
        let mut dao = DaoDaoCw4Gauge::new(mock.clone());
        dao.upload_with_cw4(mock.clone())?;
        dao.custom_gauge_setup(
            mock.clone(),
            vec![coin(100, voter1.to_string()), coin(200, voter2.to_string())],
            &[voter1.as_str(), voter2.as_str()],
        )?;
        let dao_addr = dao.dao_core.address()?;
        let test_adapter = dao.gauge_suite.test_adapter.address()?;
        let gauge_id = 0;

        // vote for option from adapter (voting members are by default
        // options in adapter in this test suite)
        dao.gauge_suite.orchestrator.call_as(&voter1).place_votes(
            gauge_id,
            Some(vec![gauge_orchestrator::state::Vote {
                option: voter1.to_string(),
                weight: Decimal::percent(90),
            }]),
        )?;

        assert_eq!(
            Some(simple_vote(
                &voter1.to_string(),
                &voter1.to_string(),
                90,
                mock.block_info()?.time.seconds()
            )),
            dao.gauge_suite
                .orchestrator
                .vote(gauge_id, voter1.to_string())?
                .vote
        );
        // check tally is proper
        let selected_set = dao.gauge_suite.orchestrator.selected_set(gauge_id)?.votes;
        assert_eq!(selected_set, vec![(voter1.to_string(), Uint128::new(90))]);
        // add new valid options to the gauge adapter
        mock.call_as(&dao_addr).execute(
            &dao_gauge_adapter::contract::ExecuteMsg::AddValidOption {
                option: "option1".into(),
            },
            &vec![],
            &test_adapter.clone(),
        )?;
        mock.call_as(&dao_addr).execute(
            &dao_gauge_adapter::contract::ExecuteMsg::AddValidOption {
                option: "option2".into(),
            },
            &vec![],
            &test_adapter.clone(),
        )?;

        // change vote for option added through gauge
        dao.gauge_suite
            .orchestrator
            .call_as(&voter1)
            .add_option(gauge_id, "option1")?;
        dao.gauge_suite
            .orchestrator
            .call_as(&voter2)
            .add_option(gauge_id, "option2")?;

        // voter2 drops vote1
        dao.gauge_suite.orchestrator.call_as(&voter2).place_votes(
            gauge_id,
            Some(vec![
                gauge_orchestrator::state::Vote {
                    option: "option1".to_string(),
                    weight: Decimal::percent(50),
                },
                gauge_orchestrator::state::Vote {
                    option: "option2".to_string(),
                    weight: Decimal::percent(50),
                },
            ]),
        )?;
        assert_eq!(
            vec![
                multi_vote(
                    &voter2.to_string(),
                    &[("option1", 50), ("option2", 50)],
                    mock.block_info()?.time.seconds(),
                ),
                simple_vote(
                    &voter1.to_string(),
                    &voter1.to_string(),
                    90,
                    mock.block_info()?.time.seconds()
                ),
            ],
            dao.gauge_suite
                .orchestrator
                .list_votes(gauge_id, None, None)?
                .votes
        );

        // execute after epoch passes
        mock.wait_seconds(EPOCH)?;
        mock.call_as(&dao_addr).execute(
            &GaugeExecuteMsg::Execute { gauge: gauge_id },
            &vec![],
            &dao.gauge_suite.orchestrator.address()?,
        )?;

        // confirm gauge recieved vote
        let pre_voter1_takeover_gauge_set =
            dao.gauge_suite.orchestrator.selected_set(gauge_id)?.votes;

        // voter1 option is least popular
        assert_eq!(
            pre_voter1_takeover_gauge_set,
            vec![
                ("option2".to_string(), Uint128::new(100)),
                ("option1".to_string(), Uint128::new(100)),
                (voter1.to_string(), Uint128::new(90)),
            ]
        );

        // Force update members, giving voter 1 more power
        mock.call_as(&dao_addr).execute(
            &Cw4ExecuteMsg::UpdateMembers {
                remove: vec![],
                add: vec![Member {
                    addr: voter1.to_string(),
                    weight: 1000,
                }],
            },
            &vec![],
            &dao.cw4_vote.group_contract()?,
        )?;
        let cw4 = dao.cw4_vote.address()?;
        dao.gauge_suite
            .orchestrator
            .call_as(&cw4)
            .member_changed_hook(MemberChangedHookMsg::new(vec![MemberDiff {
                key: voter1.to_string(),
                old: Some(100u64),
                new: Some(1000u64),
            }]))?;
        mock.wait_blocks(1)?;

        let current_gauge_set = dao.gauge_suite.orchestrator.selected_set(gauge_id)?.votes;

        // Currect selected set should be different than before voter1 got power
        assert_ne!(pre_voter1_takeover_gauge_set, current_gauge_set);

        // Voter1 option is now most popular
        assert_eq!(
            current_gauge_set,
            vec![
                (voter1.to_string(), Uint128::new(900)),
                ("option2".to_string(), Uint128::new(100)),
                ("option1".to_string(), Uint128::new(100))
            ]
        );

        // Execute after epoch passes
        mock.wait_seconds(EPOCH)?;
        mock.call_as(&dao_addr).execute(
            &GaugeExecuteMsg::Execute { gauge: gauge_id },
            &vec![],
            &dao.gauge_suite.orchestrator.address()?,
        )?;

        // Force update members, kick out voter 1
        mock.call_as(&dao_addr).execute(
            &Cw4ExecuteMsg::UpdateMembers {
                remove: vec![voter1.to_string()],
                add: vec![],
            },
            &vec![],
            &dao.cw4_vote.group_contract()?,
        )?;
        dao.gauge_suite
            .orchestrator
            .call_as(&cw4)
            .member_changed_hook(MemberChangedHookMsg::new(vec![MemberDiff {
                key: voter1.to_string(),
                old: Some(1000u64),
                new: None,
            }]))?;
        mock.wait_blocks(1)?;

        // Execute after epoch passes
        mock.wait_seconds(EPOCH)?;
        mock.call_as(&dao_addr).execute(
            &GaugeExecuteMsg::Execute { gauge: gauge_id },
            &vec![],
            &dao.gauge_suite.orchestrator.address()?,
        )?;

        let current_gauge_set = dao
            .gauge_suite
            .orchestrator
            .last_executed_set(gauge_id)?
            .votes;
        // Voter1 option is now most popular
        assert_eq!(
            current_gauge_set,
            Some(vec![
                ("option2".to_string(), Uint128::new(100)),
                ("option1".to_string(), Uint128::new(100))
            ])
        );

        Ok(())
    }
    #[test]
    fn test_token_staking_voting_power_change() -> anyhow::Result<()> {
        let mock = MockBech32::new(PREFIX);
        let voter1 = mock.addr_make_with_balance("voter1", coins(1000, "ujuno"))?;
        let voter2 = mock.addr_make_with_balance("voter2", coins(1000, "ujuno"))?;
        let mut dao = DaoDaoCw4Gauge::new(mock.clone());
        dao.upload_with_cw4(mock.clone())?;
        dao.custom_gauge_setup(
            mock.clone(),
            vec![coin(100, voter1.to_string()), coin(200, voter2.to_string())],
            &[voter1.as_str(), voter2.as_str()],
        )?;
        let dao_addr = dao.dao_core.address()?;
        let test_adapter = dao.gauge_suite.test_adapter.address()?;
        let gauge_id = 0;

        // vote for option from adapter (voting members are by default
        // options in adapter in this test suite)

        dao.gauge_suite.orchestrator.call_as(&voter1).place_votes(
            gauge_id,
            Some(vec![gauge_orchestrator::state::Vote {
                option: voter1.to_string(),
                weight: Decimal::percent(90),
            }]),
        )?;

        assert_eq!(
            Some(simple_vote(
                &voter1.to_string(),
                &voter1.to_string(),
                90,
                mock.block_info()?.time.seconds()
            )),
            dao.gauge_suite
                .orchestrator
                .vote(gauge_id, voter1.to_string())?
                .vote
        );
        // check tally is proper
        let selected_set = dao.gauge_suite.orchestrator.selected_set(gauge_id)?.votes;
        assert_eq!(selected_set, vec![(voter1.to_string(), Uint128::new(90))]);
        // add new valid options to the gauge adapter
        mock.call_as(&dao_addr).execute(
            &dao_gauge_adapter::contract::ExecuteMsg::AddValidOption {
                option: "option1".into(),
            },
            &vec![],
            &test_adapter.clone(),
        )?;
        mock.call_as(&dao_addr).execute(
            &dao_gauge_adapter::contract::ExecuteMsg::AddValidOption {
                option: "option2".into(),
            },
            &vec![],
            &test_adapter.clone(),
        )?;

        // change vote for option added through gauge
        dao.gauge_suite
            .orchestrator
            .call_as(&voter1)
            .add_option(gauge_id, "option1")?;
        dao.gauge_suite
            .orchestrator
            .call_as(&voter2)
            .add_option(gauge_id, "option2")?;

        // voter2 drops vote1
        dao.gauge_suite.orchestrator.call_as(&voter2).place_votes(
            gauge_id,
            Some(vec![
                gauge_orchestrator::state::Vote {
                    option: "option1".to_string(),
                    weight: Decimal::percent(50),
                },
                gauge_orchestrator::state::Vote {
                    option: "option2".to_string(),
                    weight: Decimal::percent(50),
                },
            ]),
        )?;
        assert_eq!(
            vec![
                multi_vote(
                    &voter2.to_string(),
                    &[("option1", 50), ("option2", 50)],
                    mock.block_info()?.time.seconds(),
                ),
                simple_vote(
                    &voter1.to_string(),
                    &voter1.to_string(),
                    90,
                    mock.block_info()?.time.seconds()
                ),
            ],
            dao.gauge_suite
                .orchestrator
                .list_votes(gauge_id, None, None)?
                .votes,
        );

        // execute after epoch passes
        mock.wait_seconds(EPOCH)?;
        mock.call_as(&dao_addr).execute(
            &GaugeExecuteMsg::Execute { gauge: gauge_id },
            &vec![],
            &dao.gauge_suite.orchestrator.address()?,
        )?;
        mock.next_block()?;

        // confirm gauge recieved vote
        let selected_set = dao.gauge_suite.orchestrator.selected_set(gauge_id)?.votes;

        // voter1 option is least popular
        assert_eq!(
            selected_set,
            vec![
                ("option2".to_string(), Uint128::new(100)),
                ("option1".to_string(), Uint128::new(100)),
                (voter1.to_string(), Uint128::new(90))
            ]
        );

        // Use hook caller to mock voter1 staking
        let cw4 = dao.cw4_vote.address()?;
        dao.gauge_suite
            .orchestrator
            .call_as(&cw4)
            .stake_change_hook(dao_hooks::stake::StakeChangedHookMsg::Stake {
                addr: voter1.clone(),
                amount: Uint128::new(900),
            })?;

        // Currect selected set should be different than before voter1 got power
        let current_gauge_set = dao.gauge_suite.orchestrator.selected_set(gauge_id)?.votes;
        assert_eq!(
            current_gauge_set,
            vec![
                (voter1.to_string(), Uint128::new(900)),
                ("option2".to_string(), Uint128::new(100)),
                ("option1".to_string(), Uint128::new(100))
            ]
        );

        // Execute after epoch passes
        mock.wait_seconds(EPOCH)?;
        mock.call_as(&dao_addr).execute(
            &GaugeExecuteMsg::Execute { gauge: gauge_id },
            &vec![],
            &dao.gauge_suite.orchestrator.address()?,
        )?;

        // Mock voter 1 unstaking
        dao.gauge_suite
            .orchestrator
            .call_as(&cw4)
            .stake_change_hook(dao_hooks::stake::StakeChangedHookMsg::Unstake {
                addr: voter1.clone(),
                amount: Uint128::new(1000),
            })?;
        mock.next_block()?;

        // Currect selected set should be different than before voter1 got power
        let current_gauge_set = dao.gauge_suite.orchestrator.selected_set(gauge_id)?.votes;
        assert_eq!(
            current_gauge_set,
            vec![
                ("option2".to_string(), Uint128::new(100)),
                ("option1".to_string(), Uint128::new(100))
            ]
        );

        Ok(())
    }
    #[test]
    fn test_nft_staking_voting_power_change() -> anyhow::Result<()> {
        let mock = MockBech32::new(PREFIX);
        let voter1 = mock.addr_make_with_balance("voter1", coins(1000, "ujuno"))?;
        let voter2 = mock.addr_make_with_balance("voter2", coins(1000, "ujuno"))?;
        let mut dao = DaoDaoCw4Gauge::new(mock.clone());
        dao.upload_with_cw4(mock.clone())?;
        dao.custom_gauge_setup(
            mock.clone(),
            vec![coin(1, voter1.to_string()), coin(1, voter2.to_string())],
            &[voter1.as_str(), voter2.as_str()],
        )?;
        let dao_addr = dao.dao_core.address()?;
        let test_adapter = dao.gauge_suite.test_adapter.address()?;
        let cw4 = dao.cw4_vote.address()?;
        let gauge_id = 0;

        // vote for option from adapter (voting members are by default
        // options in adapter in this test suite)

        dao.gauge_suite.orchestrator.call_as(&voter1).place_votes(
            gauge_id,
            Some(vec![gauge_orchestrator::state::Vote {
                option: voter1.to_string(),
                weight: Decimal::percent(100),
            }]),
        )?;

        assert_eq!(
            Some(simple_vote(
                &voter1.to_string(),
                &voter1.to_string(),
                100,
                mock.block_info()?.time.seconds()
            )),
            dao.gauge_suite
                .orchestrator
                .vote(gauge_id, voter1.to_string())?
                .vote
        );
        // check tally is proper
        let selected_set = dao.gauge_suite.orchestrator.selected_set(gauge_id)?.votes;
        assert_eq!(selected_set, vec![(voter1.to_string(), Uint128::one())]);
        // add new valid options to the gauge adapter
        mock.call_as(&dao_addr).execute(
            &dao_gauge_adapter::contract::ExecuteMsg::AddValidOption {
                option: "option1".into(),
            },
            &vec![],
            &test_adapter.clone(),
        )?;
        mock.call_as(&dao_addr).execute(
            &dao_gauge_adapter::contract::ExecuteMsg::AddValidOption {
                option: "option2".into(),
            },
            &vec![],
            &test_adapter.clone(),
        )?;

        // change vote for option added through gauge
        dao.gauge_suite
            .orchestrator
            .call_as(&voter1)
            .add_option(gauge_id, "option1")?;
        dao.gauge_suite
            .orchestrator
            .call_as(&voter2)
            .add_option(gauge_id, "option2")?;

        // voter2 drops vote1
        dao.gauge_suite.orchestrator.call_as(&voter2).place_votes(
            gauge_id,
            Some(vec![
                gauge_orchestrator::state::Vote {
                    option: "option1".to_string(),
                    weight: Decimal::percent(50),
                },
                gauge_orchestrator::state::Vote {
                    option: "option2".to_string(),
                    weight: Decimal::percent(50),
                },
            ]),
        )?;
        assert_eq!(
            vec![
                multi_vote(
                    &voter2.to_string(),
                    &[("option1", 50), ("option2", 50)],
                    mock.block_info()?.time.seconds(),
                ),
                simple_vote(
                    &voter1.to_string(),
                    &voter1.to_string(),
                    100,
                    mock.block_info()?.time.seconds()
                ),
            ],
            dao.gauge_suite
                .orchestrator
                .list_votes(gauge_id, None, None)?
                .votes,
        );

        // execute after epoch passes
        mock.wait_seconds(EPOCH)?;
        mock.call_as(&dao_addr).execute(
            &GaugeExecuteMsg::Execute { gauge: gauge_id },
            &vec![],
            &dao.gauge_suite.orchestrator.address()?,
        )?;
        mock.next_block()?;

        // confirm gauge recieved vote
        let selected_set = dao.gauge_suite.orchestrator.selected_set(gauge_id)?.votes;

        // voter1 option is least popular
        assert_eq!(
            selected_set,
            vec![
                (voter1.to_string(), Uint128::new(2)),
                ("option2".to_string(), Uint128::new(1)),
                ("option1".to_string(), Uint128::new(1)),
            ]
        );

        // Use hook caller to mock voter1 staking
        dao.gauge_suite
            .orchestrator
            .call_as(&cw4)
            .nft_stake_change_hook(NftStakeChangedHookMsg::Stake {
                addr: voter1.clone(),
                token_id: "1".to_string(),
            })?;

        mock.next_block()?;

        // Currect selected set should be different than before voter1 got power
        let current_set = dao.gauge_suite.orchestrator.selected_set(gauge_id)?.votes;

        // voter1 option is least popular
        assert_eq!(current_set, selected_set);
        assert_eq!(
            current_set,
            vec![
                ("option2".to_string(), Uint128::new(1)),
                ("option1".to_string(), Uint128::new(1)),
                (voter1.to_string(), Uint128::new(2)),
            ]
        );

        // execute after epoch passes
        mock.wait_seconds(EPOCH)?;
        mock.call_as(&dao_addr).execute(
            &GaugeExecuteMsg::Execute { gauge: gauge_id },
            &vec![],
            &dao.gauge_suite.orchestrator.address()?,
        )?;
        mock.next_block()?;

        // Mock voter1 unstaking 2 nfts

        dao.gauge_suite
            .orchestrator
            .call_as(&cw4)
            .nft_stake_change_hook(NftStakeChangedHookMsg::Unstake {
                addr: voter1.clone(),
                token_ids: vec!["1".to_string(), "2".to_string()],
            })?;
        mock.next_block()?;

        // execute after epoch passes
        mock.wait_seconds(EPOCH)?;
        mock.call_as(&dao_addr).execute(
            &GaugeExecuteMsg::Execute { gauge: gauge_id },
            &vec![],
            &dao.gauge_suite.orchestrator.address()?,
        )?;
        mock.next_block()?;

        // Currect selected set should be different than before voter1 got power
        let current_gauge_set = dao.gauge_suite.orchestrator.selected_set(gauge_id)?.votes;
        assert_eq!(
            current_gauge_set,
            vec![
                ("option2".to_string(), Uint128::new(100)),
                ("option1".to_string(), Uint128::new(100))
            ]
        );

        Ok(())
    }
    //     // todo: test on ohnft nft hooks
    //     // todo: test on bitsong fantoken hooks
    //     // todo: test on omniflix nft hooks
}
