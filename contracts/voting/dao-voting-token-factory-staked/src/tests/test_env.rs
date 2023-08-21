// The code is used in tests but reported as dead code
// see https://github.com/rust-lang/rust/issues/46379
#![allow(dead_code)]

use crate::{
    msg::{ExecuteMsg, InitialBalance, InstantiateMsg, NewTokenInfo, QueryMsg, TokenInfo},
    ContractError,
};

use cosmwasm_std::{Coin, Uint128};
use cw_tokenfactory_issuer::msg::DenomUnit;
use cw_utils::Duration;
use dao_interface::state::Admin;
use dao_testing::test_tube::cw_tokenfactory_issuer::TokenfactoryIssuer;
use osmosis_std::types::{
    cosmos::bank::v1beta1::QueryAllBalancesRequest, cosmwasm::wasm::v1::MsgExecuteContractResponse,
};
use osmosis_test_tube::{
    Account, Bank, Module, OsmosisTestApp, RunnerError, RunnerExecuteResult, RunnerResult,
    SigningAccount, Wasm,
};
use serde::de::DeserializeOwned;
use std::{collections::HashMap, path::PathBuf};

pub const DAO: &str = "dao";
pub const DENOM: &str = "ucat";
pub const JUNO: &str = "ujuno";

pub struct TestEnv<'a> {
    pub app: &'a OsmosisTestApp,
    pub creator: SigningAccount,
    pub contract: TfDaoVotingContract<'a>,
    pub tf_issuer: TokenfactoryIssuer<'a>,
    pub accounts: HashMap<String, SigningAccount>,
}

impl<'a> TestEnv<'a> {
    pub fn assert_account_balances(
        &self,
        account: &str,
        expected_balances: Vec<Coin>,
        ignore_denoms: Vec<&str>,
    ) {
        let account_balances: Vec<Coin> = Bank::new(self.app)
            .query_all_balances(&QueryAllBalancesRequest {
                address: self.accounts.get(account).unwrap().address(),
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
                address: self.contract.contract_addr.clone(),
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
    account_balances: HashMap<String, Vec<Coin>>,
    instantiate_msg: Option<InstantiateMsg>,
}

impl TestEnvBuilder {
    pub fn new() -> Self {
        Self {
            account_balances: HashMap::new(),
            instantiate_msg: None,
        }
    }

    pub fn setup(self, app: &'_ OsmosisTestApp) -> TestEnv<'_> {
        let accounts: HashMap<_, _> = self
            .account_balances
            .into_iter()
            .map(|(account, balance)| {
                let balance: Vec<_> = balance
                    .into_iter()
                    .chain(vec![Coin::new(1000000000000, "uosmo")])
                    .collect();

                (account, app.init_account(&balance).unwrap())
            })
            .collect();

        let creator = app
            .init_account(&[Coin::new(1000000000000000u128, "uosmo")])
            .unwrap();
        let issuer_id = TokenfactoryIssuer::upload(app, &creator).unwrap();

        let contract = TfDaoVotingContract::deploy(
            app,
            &InstantiateMsg {
                token_issuer_code_id: issuer_id,
                owner: Some(Admin::CoreModule {}),
                manager: Some(creator.address()),
                token_info: TokenInfo::New(NewTokenInfo {
                    subdenom: DENOM.to_string(),
                    metadata: Some(crate::msg::NewDenomMetadata {
                        description: "Awesome token, get it meow!".to_string(),
                        additional_denom_units: Some(vec![DenomUnit {
                            denom: "cat".to_string(),
                            exponent: 6,
                            aliases: vec![],
                        }]),
                        display: "cat".to_string(),
                        name: "Cat Token".to_string(),
                        symbol: "CAT".to_string(),
                    }),
                    initial_balances: vec![InitialBalance {
                        amount: Uint128::new(100),
                        address: creator.address(),
                    }],
                    initial_dao_balance: Some(Uint128::new(900)),
                }),
                unstaking_duration: Some(Duration::Height(5)),
                active_threshold: None,
            },
            &creator,
        )
        .unwrap();

        let issuer_addr =
            TfDaoVotingContract::query(&contract, &QueryMsg::TokenContract {}).unwrap();

        let tf_issuer = TokenfactoryIssuer::new_with_values(app, issuer_id, issuer_addr).unwrap();

        TestEnv {
            app,
            creator,
            contract,
            tf_issuer,
            accounts,
        }
    }

    pub fn build(self, app: &'_ OsmosisTestApp) -> TestEnv<'_> {
        let accounts: HashMap<_, _> = self
            .account_balances
            .into_iter()
            .map(|(account, balance)| {
                let balance: Vec<_> = balance
                    .into_iter()
                    .chain(vec![Coin::new(1000000000000, "uosmo")])
                    .collect();

                (account, app.init_account(&balance).unwrap())
            })
            .collect();

        let creator = app
            .init_account(&[Coin::new(1000000000000000u128, "uosmo")])
            .unwrap();

        let contract = TfDaoVotingContract::deploy(
            app,
            self.instantiate_msg
                .as_ref()
                .expect("instantiate msg not set"),
            &creator,
        )
        .unwrap();

        let issuer_addr =
            TfDaoVotingContract::query(&contract, &QueryMsg::TokenContract {}).unwrap();

        let tf_issuer = TokenfactoryIssuer::new_with_values(
            app,
            self.instantiate_msg
                .expect("instantiate msg not set")
                .token_issuer_code_id,
            issuer_addr,
        )
        .unwrap();

        TestEnv {
            app,
            creator,
            contract,
            tf_issuer,
            accounts,
        }
    }

    pub fn upload_issuer(self, app: &'_ OsmosisTestApp, signer: &SigningAccount) -> u64 {
        TokenfactoryIssuer::upload(app, signer).unwrap()
    }

    pub fn with_account(mut self, account: &str, balance: Vec<Coin>) -> Self {
        self.account_balances.insert(account.to_string(), balance);
        self
    }

    pub fn with_instantiate_msg(mut self, msg: InstantiateMsg) -> Self {
        self.instantiate_msg = Some(msg);
        self
    }
}

pub struct TfDaoVotingContract<'a> {
    pub app: &'a OsmosisTestApp,
    pub contract_addr: String,
    pub code_id: u64,
}

impl<'a> TfDaoVotingContract<'a> {
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

    pub fn execute(
        &self,
        msg: &ExecuteMsg,
        funds: &[Coin],
        signer: &SigningAccount,
    ) -> RunnerExecuteResult<MsgExecuteContractResponse> {
        let wasm = Wasm::new(self.app);
        wasm.execute(&self.contract_addr, msg, funds, signer)
    }

    pub fn query<Res>(&self, msg: &QueryMsg) -> RunnerResult<Res>
    where
        Res: ?Sized + DeserializeOwned,
    {
        let wasm = Wasm::new(self.app);
        wasm.query(&self.contract_addr, msg)
    }

    fn get_wasm_byte_code() -> Vec<u8> {
        let manifest_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        std::fs::read(
            manifest_path
                .join("..")
                .join("..")
                .join("..")
                .join("target")
                .join("wasm32-unknown-unknown")
                .join("release")
                .join("dao_voting_token_factory_staked.wasm"),
        )
        .unwrap()
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
