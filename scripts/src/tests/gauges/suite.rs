use crate::{
    tests::{daos::voting::dao_cw4_voting_template, gauges::helpers::EPOCH},
    DaoDaoCw4Gauge,
};
use cosmwasm_std::{coin, coins, to_json_binary, Decimal};
use cw4::Member;
use cw_orch::{anyhow, prelude::*};
use dao_interface::{
    msg::ExecuteMsg as CoreExecuteMsg,
    state::{Admin, ModuleInstantiateInfo},
};
use gauge_adapter::msg::AssetUnchecked;
use gauge_orchestrator::msg::{
    ExecuteMsg as GaugeExecuteMsg, ExecuteMsgFns as OrchExecuteMsgFns, GaugeConfig,
};

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
        let gauge_config = self.init_testing_adapter(&options)?;
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
        let gauge_config = self.init_testing_adapter(&default_options)?;
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

    pub fn init_testing_adapter(&self, options: &[&str]) -> anyhow::Result<GaugeConfig> {
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
    pub fn init_minimal_adapter(&self) -> anyhow::Result<GaugeConfig> {
        // init adapter
        let adapter = self.gauge_suite.adapter.instantiate(
            &gauge_adapter::msg::InstantiateMsg {
                admin: self.dao_core.address()?.to_string(),
                required_deposit: None,
                community_pool: self.dao_core.address()?.to_string(),
                reward: AssetUnchecked::new_native("ujuno", 1000u128),
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
        let adapter = self.init_testing_adapter(options)?;
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

    pub fn run_epoch(&self, mock: MockBech32, id: u64) -> anyhow::Result<()> {
        let dao = self.dao_core.address()?;
        mock.call_as(&dao).execute(
            &GaugeExecuteMsg::Execute { gauge: id },
            &vec![],
            &self.gauge_suite.orchestrator.address()?,
        )?;
        Ok(())
    }
}
