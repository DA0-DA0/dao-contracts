use cosmwasm_std::{
    to_binary, Api, Binary, Env, Extern, HandleResponse, InitResponse, Querier, StdResult, Storage,
    Uint128, WasmMsg,
};
use cw721::Cw721ExecuteMsg;
use cw721_base::InstantiateMsg;
use cw_multi_test::{App, AppResponse, Executor};
use dao_testing::contract::cw4_group_contract;

use crate::msg::{ExecuteExt, MetadataExt, QueryExt};

const DAO: &str = "dao";

// TODO add this to DAO testing after renaming
pub fn cw721_roles_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    );
    Box::new(contract)
}

pub fn setup() -> (App, Addr) {
    let app = App::default();

    let cw721_id = app.store_code(cw721_roles_contract());
    let cw721_addr = app
        .instantiate_contract(
            cw721_id,
            Addr::unchecked(DAO),
            &InstantiateMsg {
                name: "bad kids".to_string(),
                symbol: "bad kids".to_string(),
                minter: DAO.to_string(),
            },
            &[],
            "cw721_roles".to_string(),
            None,
        )
        .unwrap();

    (app, cw721_addr)
}
