use cosmwasm_std::{
    coins,
    testing::{mock_dependencies, mock_env},
    Addr, Empty, Uint128,
};
use cw_denom::UncheckedDenom;
use cw_multi_test::{App, BankSudo, Contract, ContractWrapper, Executor, SudoMsg};
use cw_payroll::{msg::InstantiateMsg as PayrollInstantiateMsg, state::UncheckedVestingParams};
use wynd_utils::Curve;

use crate::{
    contract::{migrate, CONTRACT_NAME, CONTRACT_VERSION},
    msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg},
    state::VestingContract,
};

const ALICE: &str = "alice";
const BOB: &str = "bob";
const INITIAL_BALANCE: u128 = 1000000000;
const NATIVE_DENOM: &str = "denom";

fn factory_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    )
    .with_reply(crate::contract::reply);
    Box::new(contract)
}

// fn cw20_contract() -> Box<dyn Contract<Empty>> {
//     let contract = ContractWrapper::new(
//         cw20_base::contract::execute,
//         cw20_base::contract::instantiate,
//         cw20_base::contract::query,
//     );
//     Box::new(contract)
// }

pub fn cw_payroll_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cw_payroll::contract::execute,
        cw_payroll::contract::instantiate,
        cw_payroll::contract::query,
    );
    Box::new(contract)
}

#[test]
pub fn test_instantiate_payroll_contract() {
    let mut app = App::default();
    let code_id = app.store_code(factory_contract());
    // let cw20_code_id = app.store_code(cw20_contract());
    let cw_payroll_code_id = app.store_code(cw_payroll_contract());

    // let cw20_instantiate = cw20_base::msg::InstantiateMsg {
    //     name: "DAO".to_string(),
    //     symbol: "DAO".to_string(),
    //     decimals: 6,
    //     initial_balances: vec![],
    //     mint: None,
    //     marketing: None,
    // };

    let instantiate = InstantiateMsg {};
    let factory_addr = app
        .instantiate_contract(
            code_id,
            Addr::unchecked("CREATOR"),
            &instantiate,
            &[],
            "cw-admin-factory",
            None,
        )
        .unwrap();

    // Mint alice native tokens
    app.sudo(SudoMsg::Bank({
        BankSudo::Mint {
            to_address: ALICE.to_string(),
            amount: coins(INITIAL_BALANCE, NATIVE_DENOM),
        }
    }))
    .unwrap();

    let amount = Uint128::new(1000000);
    let unchecked_denom = UncheckedDenom::Native(NATIVE_DENOM.to_string());

    // Basic linear vesting schedule
    let start_time = app.block_info().time.plus_seconds(100).seconds();
    let end_time = app.block_info().time.plus_seconds(300).seconds();
    let vesting_schedule = Curve::saturating_linear((start_time, amount.into()), (end_time, 0));

    let instantiate_payroll_msg = ExecuteMsg::InstantiatePayrollContract {
        instantiate_msg: PayrollInstantiateMsg {
            owner: Some(ALICE.to_string()),
            params: UncheckedVestingParams {
                recipient: BOB.to_string(),
                amount: Uint128::new(1000000),
                denom: unchecked_denom,
                vesting_schedule: vesting_schedule.clone(),
                title: None,
                description: None,
            },
        },
        code_id: cw_payroll_code_id,
        label: "Payroll".to_string(),
    };

    let res = app
        .execute_contract(
            Addr::unchecked(ALICE),
            factory_addr.clone(),
            &instantiate_payroll_msg,
            &coins(amount.into(), NATIVE_DENOM),
        )
        .unwrap();

    // Get the payroll address from the instantiate event
    let instantiate_event = &res.events[2];
    assert_eq!(instantiate_event.ty, "instantiate");
    let cw_payroll_addr = instantiate_event.attributes[0].value.clone();

    // Check that admin of contract is owner specified in Instantiation Message
    let contract_info = app
        .wrap()
        .query_wasm_contract_info(&cw_payroll_addr)
        .unwrap();
    assert_eq!(contract_info.admin, Some(ALICE.to_string()));

    // Test query list of contracts
    let contracts: Vec<VestingContract> = app
        .wrap()
        .query_wasm_smart(
            factory_addr.clone(),
            &QueryMsg::ListVestingContracts {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();
    assert_eq!(contracts.len(), 1);

    // Test query by instantiator
    let contracts: Vec<VestingContract> = app
        .wrap()
        .query_wasm_smart(
            factory_addr.clone(),
            &QueryMsg::ListVestingContractsByInstantiator {
                instantiator: ALICE.to_string(),
                start_after: None,
                limit: None,
            },
        )
        .unwrap();
    assert_eq!(contracts.len(), 1);

    // Test query by instantiator with no results
    let contracts: Vec<VestingContract> = app
        .wrap()
        .query_wasm_smart(
            factory_addr.clone(),
            &QueryMsg::ListVestingContractsByInstantiator {
                instantiator: BOB.to_string(),
                start_after: None,
                limit: None,
            },
        )
        .unwrap();
    assert_eq!(contracts.len(), 0);

    // Test query by recipient
    let contracts: Vec<VestingContract> = app
        .wrap()
        .query_wasm_smart(
            factory_addr.clone(),
            &QueryMsg::ListVestingContractsByRecipient {
                recipient: BOB.to_string(),
                start_after: None,
                limit: None,
            },
        )
        .unwrap();
    assert_eq!(contracts.len(), 1);

    // Test query by recipient no results
    let contracts: Vec<VestingContract> = app
        .wrap()
        .query_wasm_smart(
            factory_addr,
            &QueryMsg::ListVestingContractsByRecipient {
                recipient: ALICE.to_string(),
                start_after: None,
                limit: None,
            },
        )
        .unwrap();
    assert_eq!(contracts.len(), 0);
}

#[test]
pub fn test_migrate_update_version() {
    let mut deps = mock_dependencies();
    cw2::set_contract_version(&mut deps.storage, "my-contract", "old-version").unwrap();
    migrate(deps.as_mut(), mock_env(), MigrateMsg {}).unwrap();
    let version = cw2::get_contract_version(&deps.storage).unwrap();
    assert_eq!(version.version, CONTRACT_VERSION);
    assert_eq!(version.contract, CONTRACT_NAME);
}
