#![cfg(test)]
use cosmwasm_std::{coin, coins, Addr, BankMsg, CosmosMsg, Empty, StakingMsg};
use cw_multi_test::{App, Contract, ContractWrapper, Executor};

use crate::{
    msg::{ExecuteMsg, InstantiateMsg},
    state::Kind,
};

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
        kind: Kind::Allow {},
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
        &ExecuteMsg::AddAuthorization {
            addr: Addr::unchecked("Someone"),
            msg: r#"{"bank": {}}"#.to_string(),
        },
        &[],
    )
    .unwrap();

    app.execute_contract(
        Addr::unchecked(CREATOR),
        contract_addr.clone(),
        &ExecuteMsg::UpdateExecutedAuthorizationState {
            sender: Addr::unchecked("Someone"),
            msgs: msgs.clone(),
        },
        &[],
    )
    .unwrap();

    // No authorizations for sender
    app.execute_contract(
        Addr::unchecked(CREATOR),
        contract_addr.clone(),
        &ExecuteMsg::UpdateExecutedAuthorizationState {
            sender: Addr::unchecked("Someone_else"),
            msgs,
        },
        &[],
    )
    .unwrap_err();

    let msgs: Vec<CosmosMsg> = vec![StakingMsg::Delegate {
        validator: "validator".to_string(),
        amount: coin(1, "earth".to_string()),
    }
    .into()];

    app.execute_contract(
        Addr::unchecked(CREATOR),
        contract_addr.clone(),
        &ExecuteMsg::UpdateExecutedAuthorizationState {
            sender: Addr::unchecked("Someone"),
            msgs: msgs.clone(),
        },
        &[],
    )
    .unwrap_err();
}
