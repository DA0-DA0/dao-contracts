use cosmwasm_std::{Addr, Empty};
use cw_multi_test::{App, Contract, ContractWrapper, Executor};

use rand::seq::SliceRandom;
use rand::thread_rng;

use crate::{
    msg::{AddressItem, ExecuteMsg, QueryMsg},
    ContractError,
};

const CREATOR: &str = "CREATOR";
const ADMIN1: &str = "ADMIN1";

fn address_list_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    );
    Box::new(contract)
}

fn setup_test_case(app: &mut App, admin: &str) -> Addr {
    let address_list_id = app.store_code(address_list_contract());
    app.instantiate_contract(
        address_list_id,
        Addr::unchecked(admin),
        &crate::msg::InstantiateMsg {
            admin: Addr::unchecked(admin),
        },
        &[],
        "address-manager",
        None,
    )
    .unwrap()
}

#[test]
fn test_instantiate() {
    let mut app = App::default();
    let contract = setup_test_case(&mut app, CREATOR);

    let admin: String = app
        .wrap()
        .query_wasm_smart(contract, &QueryMsg::GetAdmin {})
        .unwrap();
    assert_eq!(admin, CREATOR.to_string())
}

#[test]
fn test_update_admin() {
    let mut app = App::default();
    let contract = setup_test_case(&mut app, CREATOR);

    app.execute_contract(
        Addr::unchecked(CREATOR),
        contract.clone(),
        &ExecuteMsg::UpdateAdmin {
            new_admin: Addr::unchecked(ADMIN1),
        },
        &[],
    )
    .unwrap();

    let admin: String = app
        .wrap()
        .query_wasm_smart(contract.clone(), &QueryMsg::GetAdmin {})
        .unwrap();
    assert_eq!(admin, ADMIN1.to_string());

    let err: ContractError = app
        .execute_contract(
            Addr::unchecked(CREATOR),
            contract,
            &ExecuteMsg::UpdateAdmin {
                new_admin: Addr::unchecked(ADMIN1),
            },
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(err, ContractError::Unauthorized {})
}

fn generate_item(priority: u32) -> AddressItem {
    AddressItem {
        addr: Addr::unchecked(format!("addr{}", priority)),
        priority,
    }
}

fn get_items_and_count(app: &App, contract: Addr) -> (u32, Vec<AddressItem>) {
    let count: u32 = app
        .wrap()
        .query_wasm_smart(contract.clone(), &QueryMsg::GetAddressCount {})
        .unwrap();
    let items: Vec<AddressItem> = app
        .wrap()
        .query_wasm_smart(contract, &QueryMsg::GetAddresses {})
        .unwrap();

    (count, items)
}

#[test]
fn test_add_remove_edge_cases() {
    let mut app = App::default();
    let contract = setup_test_case(&mut app, CREATOR);

    app.execute_contract(
            Addr::unchecked(CREATOR),
            contract.clone(),
            &ExecuteMsg::UpdateAddresses {
                to_add: vec![generate_item(10), generate_item(11)],
                to_remove: vec![generate_item(10)],
            },
            &[],
        )
        .unwrap();

    // Remove happens after add.
    let (count, items) = get_items_and_count(&app, contract.clone());
    assert_eq!(count, 1);
    assert_eq!(items, vec![generate_item(11)]);

    app.execute_contract(
        Addr::unchecked(CREATOR),
        contract.clone(),
        &ExecuteMsg::UpdateAddresses {
            to_add: vec![],
            to_remove: vec![generate_item(10)],
        },
        &[],
    ).unwrap();

    // Removing an item that doesn't exist isn't an issue.
    let (count, items) = get_items_and_count(&app, contract.clone());
    assert_eq!(count, 1);
    assert_eq!(items, vec![generate_item(11)]);

    app.execute_contract(
        Addr::unchecked(CREATOR),
        contract.clone(),
        &ExecuteMsg::UpdateAddresses {
            to_add: vec![generate_item(10), generate_item(11), generate_item(10_000)],
            to_remove: vec![generate_item(13), generate_item(10)],
        },
        &[],
    ).unwrap();

    let (count, items) = get_items_and_count(&app, contract.clone());
    assert_eq!(count, 2);
    assert_eq!(items, vec![generate_item(10_000), generate_item(11)]);
}

#[test]
fn test_add_remove() {
    let mut app = App::default();
    let contract = setup_test_case(&mut app, CREATOR);

    let (count, items) = get_items_and_count(&app, contract.clone());
    assert_eq!(count, 0);
    assert_eq!(items, vec![]);

    let mut prios: Vec<u32> = (1..500).collect();
    prios.shuffle(&mut thread_rng());
    let test_items: Vec<_> = prios.into_iter().map(generate_item).collect();

    for p in test_items.chunks(5) {
        app.execute_contract(
            Addr::unchecked(CREATOR),
            contract.clone(),
            &ExecuteMsg::UpdateAddresses {
                to_add: p.into(),
                to_remove: vec![],
            },
            &[],
        )
        .unwrap();
    }

    let (count, items) = get_items_and_count(&app, contract.clone());
    assert_eq!(count, 499);
    assert_eq!(items, (1..500).rev().map(generate_item).collect::<Vec<_>>());

    // Only the admin can add items.
    let err: ContractError = app
        .execute_contract(
            Addr::unchecked(ADMIN1),
            contract.clone(),
            &ExecuteMsg::UpdateAddresses {
                to_add: vec![generate_item(10)],
                to_remove: vec![],
            },
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(err, ContractError::Unauthorized {});

    // Remove half of the items.
    for p in test_items[0..250].chunks(50) {
        app.execute_contract(
            Addr::unchecked(CREATOR),
            contract.clone(),
            &ExecuteMsg::UpdateAddresses {
                to_add: vec![],
                to_remove: p.into(),
            },
            &[],
        )
        .unwrap();
    }

    let mut expected: Vec<AddressItem> = test_items[250..499].iter().cloned().collect();
    expected.sort();

    let (count, items) = get_items_and_count(&app, contract);
    assert_eq!(count, 249);
    assert_eq!(items, expected.into_iter().rev().collect::<Vec<_>>());
}
