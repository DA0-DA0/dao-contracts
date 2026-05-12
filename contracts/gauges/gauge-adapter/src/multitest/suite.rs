use anyhow::Result as AnyResult;
use cosmwasm_std::{coin, to_json_binary, Addr, Coin, Empty, StdResult, Uint128};
use cw20::{Cw20Coin, Cw20ExecuteMsg, MinterResponse};
use cw_denom::UncheckedDenom;
use cw_multi_test::{App, AppResponse, ContractWrapper, Executor};
use serde::de::DeserializeOwned;

use crate::{
    contract::{execute, instantiate, migrate, query},
    msg::{AdapterQueryMsg, AssetUnchecked, ExecuteMsg, InstantiateMsg},
};

pub fn adapter_contract() -> Box<dyn cw_multi_test::Contract<Empty>> {
    Box::new(ContractWrapper::new_with_empty(execute, instantiate, query).with_migrate(migrate))
}

pub fn cw20_contract() -> Box<dyn cw_multi_test::Contract<Empty>> {
    Box::new(ContractWrapper::new(
        cw20_base::contract::execute,
        cw20_base::contract::instantiate,
        cw20_base::contract::query,
    ))
}

pub fn addr(name: &str) -> Addr {
    Addr::unchecked(name)
}

/// Test harness for gauge-adapter. Wraps `cw_multi_test::App` and tracks
/// the adapter address + roles used across tests.
pub struct Suite {
    pub app: App,
    pub owner: Addr,
    pub community_pool: Addr,
    pub adapter: Addr,
    pub cw20_code_id: u64,
}

impl Suite {
    /// Adapter with native-token reward of 1_000_000 ujuno.
    pub fn new_native(required_deposit: Option<AssetUnchecked>) -> Self {
        Self::new_with_reward(
            required_deposit,
            AssetUnchecked {
                denom: UncheckedDenom::Native("juno".to_string()),
                amount: Uint128::new(1_000_000),
            },
        )
    }

    /// Adapter with a cw20-token reward. The cw20 is freshly instantiated
    /// with 1_000_000 minted to `owner`. Returns the cw20 address.
    pub fn new_cw20_reward(required_deposit: Option<AssetUnchecked>) -> (Self, Addr) {
        let mut app = App::default();
        let owner = addr("owner");
        let community_pool = addr("community_pool");

        let cw20_code_id = app.store_code(cw20_contract());
        let cw20 = instantiate_cw20(&mut app, cw20_code_id, &owner);

        let adapter_code_id = app.store_code(adapter_contract());
        let adapter = app
            .instantiate_contract(
                adapter_code_id,
                owner.clone(),
                &InstantiateMsg {
                    owner: owner.to_string(),
                    required_deposit,
                    community_pool: community_pool.to_string(),
                    reward: AssetUnchecked {
                        denom: UncheckedDenom::Cw20(cw20.to_string()),
                        amount: Uint128::new(1_000_000),
                    },
                },
                &[],
                "gauge-adapter",
                Some(owner.to_string()),
            )
            .unwrap();

        let suite = Suite {
            app,
            owner,
            community_pool,
            adapter,
            cw20_code_id,
        };
        (suite, cw20)
    }

    fn new_with_reward(required_deposit: Option<AssetUnchecked>, reward: AssetUnchecked) -> Self {
        let mut app = App::default();
        let owner = addr("owner");
        let community_pool = addr("community_pool");

        let cw20_code_id = app.store_code(cw20_contract());
        let adapter_code_id = app.store_code(adapter_contract());

        let adapter = app
            .instantiate_contract(
                adapter_code_id,
                owner.clone(),
                &InstantiateMsg {
                    owner: owner.to_string(),
                    required_deposit,
                    community_pool: community_pool.to_string(),
                    reward,
                },
                &[],
                "gauge-adapter",
                Some(owner.to_string()),
            )
            .unwrap();

        Suite {
            app,
            owner,
            community_pool,
            adapter,
            cw20_code_id,
        }
    }

    /// Adapter with a cw20 required deposit (1_000) and native juno reward.
    /// Returns the suite and the cw20 used as the required deposit. `owner`
    /// is minted 1_000_000 of the deposit cw20.
    pub fn new_cw20_deposit() -> (Self, Addr) {
        let mut app = App::default();
        let owner = addr("owner");
        let community_pool = addr("community_pool");

        let cw20_code_id = app.store_code(cw20_contract());
        let adapter_code_id = app.store_code(adapter_contract());

        let deposit_cw20 = instantiate_cw20(&mut app, cw20_code_id, &owner);

        let adapter = app
            .instantiate_contract(
                adapter_code_id,
                owner.clone(),
                &InstantiateMsg {
                    owner: owner.to_string(),
                    required_deposit: Some(AssetUnchecked {
                        denom: UncheckedDenom::Cw20(deposit_cw20.to_string()),
                        amount: Uint128::new(1_000),
                    }),
                    community_pool: community_pool.to_string(),
                    reward: AssetUnchecked {
                        denom: UncheckedDenom::Native("juno".to_string()),
                        amount: Uint128::new(1_000_000),
                    },
                },
                &[],
                "gauge-adapter",
                Some(owner.to_string()),
            )
            .unwrap();

        let suite = Suite {
            app,
            owner,
            community_pool,
            adapter,
            cw20_code_id,
        };
        (suite, deposit_cw20)
    }

    pub fn instantiate_cw20(&mut self) -> Addr {
        instantiate_cw20(&mut self.app, self.cw20_code_id, &self.owner)
    }

    pub fn execute(
        &mut self,
        sender: &Addr,
        msg: &ExecuteMsg,
        funds: &[Coin],
    ) -> AnyResult<AppResponse> {
        self.app
            .execute_contract(sender.clone(), self.adapter.clone(), msg, funds)
    }

    pub fn execute_owner(&mut self, msg: &ExecuteMsg) -> AnyResult<AppResponse> {
        let owner = self.owner.clone();
        self.execute(&owner, msg, &[])
    }

    pub fn create_submission(
        &mut self,
        sender: &Addr,
        recipient: &Addr,
        funds: Option<Coin>,
    ) -> AnyResult<AppResponse> {
        let msg = ExecuteMsg::CreateSubmission {
            name: "DAOers".to_string(),
            url: "https://daodao.zone".to_string(),
            address: recipient.to_string(),
        };
        match funds {
            Some(c) => self.execute(sender, &msg, &[c]),
            None => self.execute(sender, &msg, &[]),
        }
    }

    pub fn query<T: DeserializeOwned>(&self, msg: &AdapterQueryMsg) -> StdResult<T> {
        self.app.wrap().query_wasm_smart(&self.adapter, msg)
    }

    pub fn mint_native(&mut self, to: &Addr, amount: Coin) {
        self.app
            .sudo(cw_multi_test::SudoMsg::Bank(
                cw_multi_test::BankSudo::Mint {
                    to_address: to.to_string(),
                    amount: vec![amount],
                },
            ))
            .unwrap();
    }

    pub fn native_balance(&self, who: &Addr, denom: &str) -> Uint128 {
        self.app.wrap().query_balance(who, denom).unwrap().amount
    }

    pub fn cw20_balance(&self, cw20: &Addr, who: &Addr) -> Uint128 {
        let resp: cw20::BalanceResponse = self
            .app
            .wrap()
            .query_wasm_smart(
                cw20,
                &cw20::Cw20QueryMsg::Balance {
                    address: who.to_string(),
                },
            )
            .unwrap();
        resp.balance
    }

    pub fn cw20_send(
        &mut self,
        cw20: &Addr,
        sender: &Addr,
        contract: &Addr,
        amount: u128,
        msg: cosmwasm_std::Binary,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            sender.clone(),
            cw20.clone(),
            &Cw20ExecuteMsg::Send {
                contract: contract.to_string(),
                amount: Uint128::new(amount),
                msg,
            },
            &[],
        )
    }
}

fn instantiate_cw20(app: &mut App, code_id: u64, owner: &Addr) -> Addr {
    let msg = cw20_base::msg::InstantiateMsg {
        name: "test".to_string(),
        symbol: "TEST".to_string(),
        decimals: 6,
        initial_balances: vec![Cw20Coin {
            address: owner.to_string(),
            amount: Uint128::new(1_000_000),
        }],
        mint: Some(MinterResponse {
            minter: owner.to_string(),
            cap: None,
        }),
        marketing: None,
    };
    app.instantiate_contract(
        code_id,
        owner.clone(),
        &msg,
        &[],
        "cw20",
        Some(owner.to_string()),
    )
    .unwrap()
}

/// Helper: fund `sender` with `amount` of `denom`, then call CreateSubmission.
#[allow(dead_code)]
pub fn fund_and_submit(
    suite: &mut Suite,
    sender: &Addr,
    recipient: &Addr,
    funds: Option<Coin>,
) -> AnyResult<AppResponse> {
    if let Some(c) = funds.as_ref() {
        suite.mint_native(sender, c.clone());
    }
    suite.create_submission(sender, recipient, funds)
}

#[allow(dead_code)]
pub fn submit_cw20_create(
    suite: &mut Suite,
    cw20: &Addr,
    sender: &Addr,
    recipient: &Addr,
    amount: u128,
) -> AnyResult<AppResponse> {
    let inner = to_json_binary(&crate::msg::ReceiveMsg::CreateSubmission {
        name: "DAOers".to_string(),
        url: "https://daodao.zone".to_string(),
        address: recipient.to_string(),
    })
    .unwrap();
    suite.cw20_send(cw20, sender, &suite.adapter.clone(), amount, inner)
}

#[allow(dead_code)]
pub fn coin_u(amount: u128, denom: &str) -> Coin {
    coin(amount, denom)
}
