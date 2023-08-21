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
use dao_interface::{
    state::Admin,
    voting::{IsActiveResponse, VotingPowerAtHeightResponse},
};
use dao_testing::test_tube::cw_tokenfactory_issuer::TokenfactoryIssuer;
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
    pub vp_contract: TfDaoVotingContract<'a>,
    pub tf_issuer: TokenfactoryIssuer<'a>,
    pub accounts: Vec<SigningAccount>,
}

impl<'a> TestEnv<'a> {
    pub fn instantiate(
        &self,
        msg: &InstantiateMsg,
        signer: SigningAccount,
    ) -> Result<TfDaoVotingContract, RunnerError> {
        TfDaoVotingContract::<'a>::instantiate(self.app, self.vp_contract.code_id, msg, &signer)
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

        let vp_contract = TfDaoVotingContract::deploy(
            app,
            &InstantiateMsg {
                token_issuer_code_id: issuer_id,
                owner: Some(Admin::CoreModule {}),
                manager: Some(accounts[0].address()),
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
                    initial_balances,
                    initial_dao_balance: Some(Uint128::new(900)),
                }),
                unstaking_duration: Some(Duration::Time(2)),
                active_threshold: Some(ActiveThreshold::AbsoluteCount {
                    count: Uint128::new(75),
                }),
            },
            &accounts[0],
        )
        .unwrap();

        let issuer_addr =
            TfDaoVotingContract::query(&vp_contract, &QueryMsg::TokenContract {}).unwrap();

        let tf_issuer = TokenfactoryIssuer::new_with_values(app, issuer_id, issuer_addr).unwrap();

        TestEnv {
            app,
            vp_contract,
            tf_issuer,
            accounts,
        }
    }

    pub fn build(self, app: &'_ OsmosisTestApp) -> TestEnv<'_> {
        let accounts = self.accounts;

        let vp_contract = TfDaoVotingContract::deploy(
            app,
            self.instantiate_msg
                .as_ref()
                .expect("instantiate msg not set"),
            &accounts[0],
        )
        .unwrap();

        let issuer_addr =
            TfDaoVotingContract::query(&vp_contract, &QueryMsg::TokenContract {}).unwrap();

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
            vp_contract,
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

#[derive(Debug)]
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

    pub fn query_active(&self) -> RunnerResult<IsActiveResponse> {
        self.query(&QueryMsg::IsActive {})
    }

    pub fn query_denom(&self) -> RunnerResult<DenomResponse> {
        self.query(&QueryMsg::Denom {})
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

    fn get_wasm_byte_code() -> Vec<u8> {
        let manifest_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let byte_code = std::fs::read(
            manifest_path
                .join("..")
                .join("..")
                .join("..")
                .join("artifacts")
                .join("dao_voting_token_factory_staked.wasm"),
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
                    .join("dao_voting_token_factory_staked-aarch64.wasm"),
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
