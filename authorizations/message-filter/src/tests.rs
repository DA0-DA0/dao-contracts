#![cfg(test)]
use cosmwasm_std::{coins, Addr, BankMsg, CosmosMsg, Empty};
use cw_multi_test::{App, Contract, ContractWrapper, Executor};
use schemars::_serde_json::json;

use crate::msg::{ExecuteMsg, InstantiateMsg};

fn contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    );
    Box::new(contract)
}

const CREATOR: &str = "creator";

#[test]
fn test_simple_filtering() {
    let mut app = App::default();

    // Create a message-filter contract
    let code_id = app.store_code(contract());
    let instantiate_msg = InstantiateMsg {
        dao: Addr::unchecked(CREATOR),
    };
    let contract_addr = app
        .instantiate_contract(
            code_id,
            Addr::unchecked(CREATOR),
            &instantiate_msg,
            &[],
            "Message Filter",
            None,
        )
        .unwrap();

    // Generate a bank message
    let to_address = String::from("you");
    let amount = coins(1015, "earth");
    let bank = BankMsg::Send { to_address, amount };
    let msg: CosmosMsg = bank.clone().into();

    let msgs = vec![msg];

    app.execute_contract(
        Addr::unchecked(CREATOR),
        contract_addr.clone(),
        &ExecuteMsg::AllowMessages { msgs },
        &[],
    )
    .unwrap();
}
