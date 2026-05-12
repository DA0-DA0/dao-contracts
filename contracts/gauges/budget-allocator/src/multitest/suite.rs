use anyhow::Result as AnyResult;
use cosmwasm_std::{coin, Addr, Coin, Empty, StdResult, Uint128};
use cw_multi_test::{App, AppResponse, ContractWrapper, Executor};
use serde::de::DeserializeOwned;

use crate::{
    contract::{execute, instantiate, migrate, query},
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
};

pub fn contract() -> Box<dyn cw_multi_test::Contract<Empty>> {
    Box::new(ContractWrapper::new_with_empty(execute, instantiate, query).with_migrate(migrate))
}

pub fn addr(name: &str) -> Addr {
    Addr::unchecked(name)
}

pub struct Suite {
    pub app: App,
    pub admin: Addr,
    pub allocator: Addr,
}

impl Suite {
    pub fn new(options: &[&str], epoch_budget: Coin) -> Self {
        let mut app = App::default();
        let admin = addr("admin");

        let code_id = app.store_code(contract());
        let allocator = app
            .instantiate_contract(
                code_id,
                admin.clone(),
                &InstantiateMsg {
                    admin: admin.to_string(),
                    options: options.iter().map(|s| s.to_string()).collect(),
                    epoch_budget,
                },
                &[],
                "budget-allocator",
                Some(admin.to_string()),
            )
            .unwrap();

        Suite {
            app,
            admin,
            allocator,
        }
    }

    pub fn execute_as(&mut self, sender: &Addr, msg: &ExecuteMsg) -> AnyResult<AppResponse> {
        self.app
            .execute_contract(sender.clone(), self.allocator.clone(), msg, &[])
    }

    pub fn execute_admin(&mut self, msg: &ExecuteMsg) -> AnyResult<AppResponse> {
        let admin = self.admin.clone();
        self.execute_as(&admin, msg)
    }

    pub fn query<T: DeserializeOwned>(&self, msg: &QueryMsg) -> StdResult<T> {
        self.app.wrap().query_wasm_smart(&self.allocator, msg)
    }
}

pub fn ujuno(amount: u128) -> Coin {
    coin(amount, "ujuno")
}

pub fn raw(amount: u128) -> Uint128 {
    Uint128::new(amount)
}
