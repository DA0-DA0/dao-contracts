use cosmwasm_std::{Addr, Empty, to_json_binary};
use cw_multi_test::{App, Contract, ContractWrapper, Executor, SudoMsg, WasmSudo};

use crate::msg::InstantiateMsg;

fn cosmos_staking_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    );
    Box::new(contract)
}

const DAO: &str = "dao";
const DELEGATOR: &str = "delegator";
const VALIDATOR: &str = "validator";

#[test]
fn happy_path() {
    let mut app = App::default();

    let cosmos_staking_code_id = app.store_code(cosmos_staking_contract());

    let vp_contract = app
        .instantiate_contract(
            cosmos_staking_code_id,
            Addr::unchecked(DAO),
            &InstantiateMsg {},
            &[],
            "cosmos_voting_power_contract",
            None,
        )
        .unwrap();

    // Manually update a delegation, normally this would be called by cw-hooks
    app.sudo(SudoMsg::Wasm(WasmSudo {
        contract_addr: vp_contract,
        msg: to_json_binary(&crate::msg::SudoMsg::BeforeDelegationSharesModified {
            validator_address: VALIDATOR.to_string(),
            delegator_address: DELEGATOR.to_string(),
            shares: "100000".to_string(),
        }).unwrap()
    })).unwrap();
}
