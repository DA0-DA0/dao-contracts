use cosmwasm_std::testing::{MockApi, MockStorage};
use cosmwasm_std::{to_binary, Addr, Api, Empty, Storage, Uint128};
use cw_multi_test::{AppBuilder, Executor, Router};
use cw_multi_test::{Contract, ContractWrapper};
use cw_verifier_middleware::msg::Payload;
use cw_verifier_middleware::utils::get_wrapped_msg;

use crate::msg::{ExecuteMsg, InnerExecuteMsg};

fn test_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    );
    Box::new(contract)
}

fn no_init<BankT, CustomT, WasmT, StakingT, DistrT, IbcT, GovT>(
    _: &mut Router<BankT, CustomT, WasmT, StakingT, DistrT, IbcT, GovT>,
    _: &dyn Api,
    _: &mut dyn Storage,
) {
}

#[test]
fn test_verify() {
    let api = MockApi::default();
    let storage = MockStorage::new();

    let mut app = AppBuilder::new()
        .with_api(api)
        .with_storage(storage)
        .build(no_init);

    let code_id = app.store_code(test_contract());
    let contract = app
        .instantiate_contract(
            code_id,
            Addr::unchecked("admin"),
            &crate::msg::InstantiateMsg {},
            &[],
            "test contract",
            None,
        )
        .unwrap();

    let payload = Payload {
        nonce: Uint128::from(0u128),
        msg: to_binary(&InnerExecuteMsg::Execute {}).unwrap(),
        expiration: None,
        contract_address: Addr::unchecked("contract_address").to_string(),
        bech32_prefix: "juno".to_string(),
        contract_version: "version-1".to_string(),
        chain_id: "juno-1".to_string(),
    };

    let wrapped_msg = get_wrapped_msg(&api, payload);
    app.execute_contract(
        Addr::unchecked("ADMIN"),
        contract,
        &ExecuteMsg {
            wrapped_msg: wrapped_msg,
        },
        &[],
    )
    .unwrap();
}
