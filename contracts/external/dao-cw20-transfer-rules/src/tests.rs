use cosmwasm_std::Empty;
use cw_multi_test::{App, Contract, ContractWrapper};

fn dao_cw20_transfer_rules_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    );
    Box::new(contract)
}

fn cw20_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cw20_base::contract::execute,
        cw20_base::contract::instantiate,
        cw20_base::contract::query,
    );
    Box::new(contract)
}

#[test]
pub fn test() {
    let mut app = App::default();
    let _dao_cw20_transfer_rules_code_id = app.store_code(dao_cw20_transfer_rules_contract());
    let _cw20_code_id = app.store_code(cw20_contract());
    // let cw20_instantiate = cw20_base::msg::InstantiateMsg {
    //     name: "DAO".to_string(),
    //     symbol: "DAO".to_string(),
    //     decimals: 6,
    //     initial_balances: vec![],
    //     mint: None,
    //     marketing: None,
    // };

    // let instantiate = InstantiateMsg {};
    // let factory_addr = app
    //     .instantiate_contract(
    //         code_id,
    //         Addr::unchecked("CREATOR"),
    //         &instantiate,
    //         &[],
    //         "cw-admin-factory",
    //         None,
    //     )
    //     .unwrap();
}
