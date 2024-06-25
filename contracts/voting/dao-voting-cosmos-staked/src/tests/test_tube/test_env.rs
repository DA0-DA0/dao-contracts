// The code is used in tests but reported as dead code
// see https://github.com/rust-lang/rust/issues/46379
#![allow(dead_code)]

use crate::{
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
    ContractError,
};

use cosmwasm_std::{to_json_binary, Addr, Coin, Decimal};
use cw_utils::Duration;
use dao_interface::{
    msg::QueryMsg as DaoQueryMsg,
    state::{Admin, ModuleInstantiateInfo, ProposalModule},
    voting::{TotalPowerAtHeightResponse, VotingPowerAtHeightResponse},
};
use dao_voting::{
    pre_propose::PreProposeInfo, threshold::PercentageThreshold, threshold::Threshold,
};

use dao_testing::test_tube::{
    dao_dao_core::DaoCore, dao_proposal_single::DaoProposalSingle,
    dao_test_custom_factory::CustomFactoryContract,
};
use osmosis_test_tube::{
    osmosis_std::types::{
        cosmos::bank::v1beta1::QueryAllBalancesRequest,
        cosmwasm::wasm::v1::MsgExecuteContractResponse,
    },
    Account, Bank, Module, OsmosisTestApp, RunnerError, RunnerExecuteResult, RunnerResult,
    SigningAccount, Wasm,
};
use serde::de::DeserializeOwned;
use std::path::PathBuf;

pub struct TestEnv<'a> {
    pub app: &'a OsmosisTestApp,
    pub dao: Option<DaoCore<'a>>,
    pub proposal_single: Option<DaoProposalSingle<'a>>,
    pub custom_factory: Option<CustomFactoryContract<'a>>,
    pub vp_contract: TokenVotingContract<'a>,
    pub accounts: Vec<SigningAccount>,
}

impl<'a> TestEnv<'a> {
    pub fn instantiate(
        &self,
        msg: &InstantiateMsg,
        signer: SigningAccount,
    ) -> Result<TokenVotingContract, RunnerError> {
        TokenVotingContract::<'a>::instantiate(self.app, self.vp_contract.code_id, msg, &signer)
    }

    pub fn bank(&self) -> Bank<'_, OsmosisTestApp> {
        Bank::new(self.app)
    }

    pub fn assert_account_balances(
        &self,
        account: SigningAccount,
        expected_balances: Vec<Coin>,
        ignore_denoms: Vec<&str>,
    ) {
        let account_balances: Vec<Coin> = Bank::new(self.app)
            .query_all_balances(&QueryAllBalancesRequest {
                address: account.address(),
                pagination: None,
            })
            .unwrap()
            .balances
            .into_iter()
            .map(|coin| Coin::new(coin.amount.parse().unwrap(), coin.denom))
            .filter(|coin| !ignore_denoms.contains(&coin.denom.as_str()))
            .collect();

        assert_eq!(account_balances, expected_balances);
    }

    pub fn assert_contract_balances(&self, expected_balances: &[Coin]) {
        let contract_balances: Vec<Coin> = Bank::new(self.app)
            .query_all_balances(&QueryAllBalancesRequest {
                address: self.vp_contract.contract_addr.clone(),
                pagination: None,
            })
            .unwrap()
            .balances
            .into_iter()
            .map(|coin| Coin::new(coin.amount.parse().unwrap(), coin.denom))
            .collect();

        assert_eq!(contract_balances, expected_balances);
    }
}

pub struct TestEnvBuilder {
    pub accounts: Vec<SigningAccount>,
    pub instantiate_msg: Option<InstantiateMsg>,
}

impl TestEnvBuilder {
    pub fn new() -> Self {
        Self {
            accounts: vec![],
            instantiate_msg: None,
        }
    }

    // Minimal default setup with just the key contracts
    pub fn default_setup(self, app: &'_ OsmosisTestApp) -> TestEnv<'_> {
        let accounts = app
            .init_accounts(&[Coin::new(1000000000000000u128, "uosmo")], 10)
            .unwrap();

        let vp_contract =
            TokenVotingContract::deploy(app, &InstantiateMsg {}, &accounts[0]).unwrap();

        TestEnv {
            app,
            accounts,
            dao: None,
            proposal_single: None,
            custom_factory: None,
            vp_contract,
        }
    }

    // Full DAO setup
    pub fn full_dao_setup(self, app: &'_ OsmosisTestApp) -> TestEnv<'_> {
        let accounts = app
            .init_accounts(&[Coin::new(1000000000000000u128, "uosmo")], 10)
            .unwrap();

        // Upload all needed code ids
        let vp_contract_id = TokenVotingContract::upload(app, &accounts[0]).unwrap();
        let proposal_single_id = DaoProposalSingle::upload(app, &accounts[0]).unwrap();

        let msg = dao_interface::msg::InstantiateMsg {
            dao_uri: None,
            admin: None,
            name: "DAO DAO".to_string(),
            description: "A DAO that makes DAO tooling".to_string(),
            image_url: None,
            automatically_add_cw20s: false,
            automatically_add_cw721s: false,
            voting_module_instantiate_info: ModuleInstantiateInfo {
                code_id: vp_contract_id,
                msg: to_json_binary(&InstantiateMsg {}).unwrap(),
                admin: Some(Admin::CoreModule {}),
                funds: vec![],
                label: "DAO DAO Voting Module".to_string(),
            },
            proposal_modules_instantiate_info: vec![ModuleInstantiateInfo {
                code_id: proposal_single_id,
                msg: to_json_binary(&dao_proposal_single::msg::InstantiateMsg {
                    min_voting_period: None,
                    threshold: Threshold::ThresholdQuorum {
                        threshold: PercentageThreshold::Percent(Decimal::percent(1)),
                        quorum: PercentageThreshold::Percent(Decimal::percent(1)),
                    },
                    max_voting_period: Duration::Time(432000),
                    allow_revoting: false,
                    only_members_execute: true,
                    close_proposal_on_execution_failure: false,
                    pre_propose_info: PreProposeInfo::AnyoneMayPropose {},
                    veto: None,
                })
                .unwrap(),
                admin: Some(Admin::CoreModule {}),
                funds: vec![],
                label: "DAO DAO Proposal Module".to_string(),
            }],
            initial_items: None,
        };

        // Instantiate DAO
        let dao = DaoCore::new(app, &msg, &accounts[0], &[]).unwrap();

        // Get voting module address, setup vp_contract helper
        let vp_addr: Addr = dao.query(&DaoQueryMsg::VotingModule {}).unwrap();
        let vp_contract =
            TokenVotingContract::new_with_values(app, vp_contract_id, vp_addr.to_string()).unwrap();

        // Get proposal module address, setup proposal_single helper
        let proposal_modules: Vec<ProposalModule> = dao
            .query(&DaoQueryMsg::ProposalModules {
                limit: None,
                start_after: None,
            })
            .unwrap();
        let proposal_single = DaoProposalSingle::new_with_values(
            app,
            proposal_single_id,
            proposal_modules[0].address.to_string(),
        )
        .unwrap();

        // Instantiate Custom Factory
        let custom_factory = CustomFactoryContract::new(
            app,
            &dao_test_custom_factory::msg::InstantiateMsg {},
            &accounts[0],
        )
        .unwrap();

        TestEnv {
            app,
            dao: Some(dao),
            vp_contract,
            proposal_single: Some(proposal_single),
            custom_factory: Some(custom_factory),
            accounts,
        }
    }

    pub fn set_accounts(mut self, accounts: Vec<SigningAccount>) -> Self {
        self.accounts = accounts;
        self
    }

    pub fn with_account(mut self, account: SigningAccount) -> Self {
        self.accounts.push(account);
        self
    }

    pub fn with_instantiate_msg(mut self, msg: InstantiateMsg) -> Self {
        self.instantiate_msg = Some(msg);
        self
    }
}

#[derive(Debug)]
pub struct TokenVotingContract<'a> {
    pub app: &'a OsmosisTestApp,
    pub contract_addr: String,
    pub code_id: u64,
}

impl<'a> TokenVotingContract<'a> {
    pub fn deploy(
        app: &'a OsmosisTestApp,
        instantiate_msg: &InstantiateMsg,
        signer: &SigningAccount,
    ) -> Result<Self, RunnerError> {
        let wasm = Wasm::new(app);

        let code_id = wasm
            .store_code(&Self::get_wasm_byte_code(), None, signer)?
            .data
            .code_id;

        let contract_addr = wasm
            .instantiate(
                code_id,
                &instantiate_msg,
                Some(&signer.address()),
                None,
                &[],
                signer,
            )?
            .data
            .address;

        Ok(Self {
            app,
            code_id,
            contract_addr,
        })
    }

    pub fn new_with_values(
        app: &'a OsmosisTestApp,
        code_id: u64,
        contract_addr: String,
    ) -> Result<Self, RunnerError> {
        Ok(Self {
            app,
            code_id,
            contract_addr,
        })
    }

    /// uploads contract and returns a code ID
    pub fn upload(app: &OsmosisTestApp, signer: &SigningAccount) -> Result<u64, RunnerError> {
        let wasm = Wasm::new(app);

        let code_id = wasm
            .store_code(&Self::get_wasm_byte_code(), None, signer)?
            .data
            .code_id;

        Ok(code_id)
    }

    pub fn instantiate(
        app: &'a OsmosisTestApp,
        code_id: u64,
        instantiate_msg: &InstantiateMsg,
        signer: &SigningAccount,
    ) -> Result<Self, RunnerError> {
        let wasm = Wasm::new(app);
        let contract_addr = wasm
            .instantiate(
                code_id,
                &instantiate_msg,
                Some(&signer.address()),
                None,
                &[],
                signer,
            )?
            .data
            .address;

        Ok(Self {
            app,
            code_id,
            contract_addr,
        })
    }

    pub fn execute(
        &self,
        msg: &ExecuteMsg,
        funds: &[Coin],
        signer: &SigningAccount,
    ) -> RunnerExecuteResult<MsgExecuteContractResponse> {
        let wasm = Wasm::new(self.app);
        wasm.execute(&self.contract_addr, msg, funds, signer)
    }

    pub fn query<T>(&self, msg: &QueryMsg) -> RunnerResult<T>
    where
        T: ?Sized + DeserializeOwned,
    {
        let wasm = Wasm::new(self.app);
        wasm.query(&self.contract_addr, msg)
    }

    pub fn query_vp(
        &self,
        address: &str,
        height: Option<u64>,
    ) -> RunnerResult<VotingPowerAtHeightResponse> {
        self.query(&QueryMsg::VotingPowerAtHeight {
            address: address.to_string(),
            height,
        })
    }

    pub fn query_tp(&self, height: Option<u64>) -> RunnerResult<TotalPowerAtHeightResponse> {
        self.query(&QueryMsg::TotalPowerAtHeight { height })
    }

    fn get_wasm_byte_code() -> Vec<u8> {
        let manifest_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let byte_code = std::fs::read(
            manifest_path
                .join("..")
                .join("..")
                .join("..")
                .join("artifacts")
                .join("dao_voting_cosmos_staked.wasm"),
        );
        match byte_code {
            Ok(byte_code) => byte_code,
            // On arm processors, the above path is not found, so we try the following path
            Err(_) => std::fs::read(
                manifest_path
                    .join("..")
                    .join("..")
                    .join("..")
                    .join("artifacts")
                    .join("dao_voting_cosmos_staked-aarch64.wasm"),
            )
            .unwrap(),
        }
    }

    pub fn execute_error(err: ContractError) -> RunnerError {
        RunnerError::ExecuteError {
            msg: format!(
                "failed to execute message; message index: 0: {}: execute wasm contract failed",
                err
            ),
        }
    }

    pub fn execute_submessage_error(err: ContractError) -> RunnerError {
        RunnerError::ExecuteError {
            msg: format!(
                "failed to execute message; message index: 0: dispatch: submessages: reply: {}: execute wasm contract failed",
                err
            ),
        }
    }
}

pub fn assert_contract_err(expected: ContractError, actual: RunnerError) {
    match actual {
        RunnerError::ExecuteError { msg } => {
            if !msg.contains(&expected.to_string()) {
                panic!(
                    "assertion failed:\n\n  must contain \t: \"{}\",\n  actual \t: \"{}\"\n",
                    expected, msg
                )
            }
        }
        _ => panic!("unexpected error, expect execute error but got: {}", actual),
    };
}
