// The code is used in tests but reported as dead code
// see https://github.com/rust-lang/rust/issues/46379
#![allow(dead_code)]

use crate::{
    msg::{ExecuteMsg, InitialBalance, InstantiateMsg, NewTokenInfo, QueryMsg, TokenInfo},
    ContractError,
};

use cosmwasm_std::{Coin, Uint128};
use cw_tokenfactory_issuer::msg::{DenomResponse, DenomUnit};
use cw_utils::Duration;
use dao_interface::voting::{IsActiveResponse, VotingPowerAtHeightResponse};
use dao_testing::test_tube::{cw_abc::CwAbc, cw_tokenfactory_issuer::TokenfactoryIssuer};
use dao_voting::threshold::ActiveThreshold;
use osmosis_std::types::{
    cosmos::bank::v1beta1::QueryAllBalancesRequest, cosmwasm::wasm::v1::MsgExecuteContractResponse,
};
use osmosis_test_tube::{
    Account, Bank, Module, OsmosisTestApp, RunnerError, RunnerExecuteResult, RunnerResult,
    SigningAccount, Wasm,
};
use serde::de::DeserializeOwned;
use std::path::PathBuf;

pub const DENOM: &str = "ucat";
pub const JUNO: &str = "ujuno";

pub struct TestEnv<'a> {
    pub app: &'a OsmosisTestApp,
    pub abc: TfDaoVotingContract<'a>,
    pub tf_issuer: TokenfactoryIssuer<'a>,
    pub accounts: Vec<SigningAccount>,
}

impl<'a> TestEnv<'a> {
    pub fn instantiate(
        &self,
        msg: &InstantiateMsg,
        signer: SigningAccount,
    ) -> Result<CwAbc, RunnerError> {
        CwAbc::<'a>::instantiate(self.app, self.abc.code_id, msg, &signer)
    }

    pub fn get_tf_issuer_code_id(&self) -> u64 {
        self.tf_issuer.code_id
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
                address: self.abc.contract_addr.clone(),
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

    pub fn default_setup(self, app: &'_ OsmosisTestApp) -> TestEnv<'_> {
        let accounts = app
            .init_accounts(&[Coin::new(1000000000000000u128, "uosmo")], 10)
            .unwrap();

        let initial_balances: Vec<InitialBalance> = accounts
            .iter()
            .map(|acc| InitialBalance {
                address: acc.address(),
                amount: Uint128::new(100),
            })
            .collect();

        let issuer_id = TokenfactoryIssuer::upload(app, &accounts[0]).unwrap();

        let abc = CwAbc::deploy(app, &InstantiateMsg {}, &accounts[0]).unwrap();

        let issuer_addr = CwAbc::query(&abc, &QueryMsg::TokenContract {}).unwrap();

        let tf_issuer = TokenfactoryIssuer::new_with_values(app, issuer_id, issuer_addr).unwrap();

        TestEnv {
            app,
            abc,
            tf_issuer,
            accounts,
        }
    }

    pub fn build(self, app: &'_ OsmosisTestApp) -> TestEnv<'_> {
        let accounts = self.accounts;

        let abc = CwAbc::deploy(
            app,
            self.instantiate_msg
                .as_ref()
                .expect("instantiate msg not set"),
            &accounts[0],
        )
        .unwrap();

        let issuer_addr = CwAbc::query(&abc, &QueryMsg::TokenContract {}).unwrap();

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
            abc,
            tf_issuer,
            accounts,
        }
    }

    pub fn upload_issuer(self, app: &'_ OsmosisTestApp, signer: &SigningAccount) -> u64 {
        TokenfactoryIssuer::upload(app, signer).unwrap()
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
