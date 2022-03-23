use crate::{
    msg::{DumpResponse, Group, InstantiateMsg, QueryMsg},
    ContractError,
};
use cosmwasm_std::{Addr, Empty};
use cw_multi_test::{App, Contract, ContractWrapper, Executor};

fn named_group_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    );
    Box::new(contract)
}

const ADMIN: &str = "DAO";
const USER1: &str = "USER1";
const USER2: &str = "USER2";

fn group_factory(id: u8) -> Group {
    Group {
        name: format!("GROUP{}_NAME", id),
        addresses: vec![format!("USER{}", id)],
    }
}

fn mock_app() -> App {
    App::default()
}

fn instantiate(groups: Option<Vec<Group>>) -> Result<(App, Addr), ContractError> {
    let mut app = mock_app();
    let contract_id = app.store_code(named_group_contract());

    let msg = InstantiateMsg { groups };
    let instantiation_result =
        app.instantiate_contract(contract_id, Addr::unchecked(ADMIN), &msg, &[], "test", None);

    if let Ok(contract_addr) = instantiation_result {
        Ok((app, contract_addr))
    } else {
        Err(instantiation_result.unwrap_err().downcast().unwrap())
    }
}

fn dump(app: &App, contract_addr: &Addr) -> DumpResponse {
    app.wrap()
        .query_wasm_smart(contract_addr, &QueryMsg::Dump {})
        .unwrap()
}

mod instantiate {
    use super::*;

    #[test]
    fn instantiate_with_no_groups() {
        let (app, contract_addr) = instantiate(None).unwrap();

        // Ensure there are no groups.
        let dump_result = dump(&app, &contract_addr);
        assert_eq!(dump_result.groups.len(), 0);
    }

    #[test]
    fn instantiate_with_groups() {
        let groups = vec![group_factory(1), group_factory(2), group_factory(3)];

        let (app, contract_addr) = instantiate(Some(groups.clone())).unwrap();

        // Ensure there are the expected groups.
        let dump_result = dump(&app, &contract_addr);
        assert_eq!(dump_result.groups, groups);
    }
}

mod add {
    use crate::msg::ExecuteMsg;

    use super::*;

    #[test]
    fn add_unauthorized() {
        let (mut app, contract_addr) = instantiate(None).unwrap();

        // Try to add a new group.
        let err: ContractError = app
            .execute_contract(
                Addr::unchecked(USER1),
                contract_addr.clone(),
                &ExecuteMsg::Add {
                    group: "group1".to_string(),
                    addresses: vec![USER1.to_string()],
                },
                &[],
            )
            .unwrap_err()
            .downcast()
            .unwrap();
        // Expect unauthorized.
        assert_eq!(err, ContractError::Unauthorized {});

        // Ensure there are no groups.
        let dump_result = dump(&app, &contract_addr);
        assert_eq!(dump_result.groups.len(), 0);
    }

    #[test]
    fn add_to_new_group() {
        let (mut app, contract_addr) = instantiate(None).unwrap();

        let group1 = group_factory(1);

        // Add a new group.
        app.execute_contract(
            Addr::unchecked(ADMIN),
            contract_addr.clone(),
            &ExecuteMsg::Add {
                group: group1.name.clone(),
                addresses: group1.addresses.clone(),
            },
            &[],
        )
        .unwrap();

        // Ensure there is one group with the expected contents.
        let dump_result = dump(&app, &contract_addr);
        assert_eq!(dump_result.groups.len(), 1);
        assert_eq!(dump_result.groups[0], group1);
    }

    #[test]
    fn add_to_existing_group() {
        let (mut app, contract_addr) = instantiate(None).unwrap();

        let group_name = "GROUP_NAME".to_string();
        let addresses = vec![USER1.to_string(), USER2.to_string()];

        // Add a new group.
        app.execute_contract(
            Addr::unchecked(ADMIN),
            contract_addr.clone(),
            &ExecuteMsg::Add {
                group: group_name.clone(),
                addresses: addresses[..1].to_vec(),
            },
            &[],
        )
        .unwrap();

        // Add to the existing group.
        app.execute_contract(
            Addr::unchecked(ADMIN),
            contract_addr.clone(),
            &ExecuteMsg::Add {
                group: group_name.clone(),
                addresses: addresses[1..].to_vec(),
            },
            &[],
        )
        .unwrap();

        // Ensure there is one group with the expected contents.
        let dump_result = dump(&app, &contract_addr);
        assert_eq!(dump_result.groups.len(), 1);
        assert_eq!(
            dump_result.groups[0],
            Group {
                name: group_name,
                addresses,
            }
        );
    }

    #[test]
    fn add_to_two_groups() {
        let (mut app, contract_addr) = instantiate(None).unwrap();

        let group1 = group_factory(1);
        let group2 = group_factory(2);

        // Add a new group.
        app.execute_contract(
            Addr::unchecked(ADMIN),
            contract_addr.clone(),
            &ExecuteMsg::Add {
                group: group1.name.clone(),
                addresses: group1.addresses.clone(),
            },
            &[],
        )
        .unwrap();

        // Add another new group.
        app.execute_contract(
            Addr::unchecked(ADMIN),
            contract_addr.clone(),
            &ExecuteMsg::Add {
                group: group2.name.clone(),
                addresses: group2.addresses.clone(),
            },
            &[],
        )
        .unwrap();

        // Ensure there are two groups with the expected contents.
        let dump_result = dump(&app, &contract_addr);
        assert_eq!(dump_result.groups.len(), 2);
        // Sort to ensure deterministic ordering.
        let mut dump_groups = dump_result.groups;
        dump_groups.sort_by_key(|group| group.name.clone());
        assert_eq!(dump_groups[0], group1);
        assert_eq!(dump_groups[1], group2);
    }
}

mod remove {
    use crate::msg::ExecuteMsg;

    use super::*;

    #[test]
    fn remove_unauthorized() {
        let group1 = group_factory(1);

        let (mut app, contract_addr) = instantiate(Some(vec![group1.clone()])).unwrap();

        // Try to remove a group.
        let err: ContractError = app
            .execute_contract(
                Addr::unchecked(USER1),
                contract_addr.clone(),
                &ExecuteMsg::Remove {
                    group: group1.name.clone(),
                    addresses: Some(group1.addresses.clone()),
                },
                &[],
            )
            .unwrap_err()
            .downcast()
            .unwrap();
        // Expect unauthorized.
        assert_eq!(err, ContractError::Unauthorized {});

        // Ensure there is still one group with the expected contents.
        let dump_result = dump(&app, &contract_addr);
        assert_eq!(dump_result.groups.len(), 1);
        assert_eq!(dump_result.groups[0], group1);
    }

    #[test]
    fn remove_from_nonexistent_group() {
        let (mut app, contract_addr) = instantiate(None).unwrap();

        let group1 = group_factory(1);

        // Try to remove a non-existent group with no addresses.
        let err: ContractError = app
            .execute_contract(
                Addr::unchecked(ADMIN),
                contract_addr.clone(),
                &ExecuteMsg::Remove {
                    group: group1.name.clone(),
                    addresses: None,
                },
                &[],
            )
            .unwrap_err()
            .downcast()
            .unwrap();
        // Expect invalid group.
        assert_eq!(err, ContractError::InvalidGroup(group1.name.clone()));

        // Try to remove a non-existent group with some addresses.
        let err: ContractError = app
            .execute_contract(
                Addr::unchecked(ADMIN),
                contract_addr,
                &ExecuteMsg::Remove {
                    group: group1.name.clone(),
                    addresses: Some(group1.addresses.clone()),
                },
                &[],
            )
            .unwrap_err()
            .downcast()
            .unwrap();
        // Expect invalid group.
        assert_eq!(err, ContractError::InvalidGroup(group1.name));
    }

    #[test]
    fn remove_from_existing_group() {
        let group1 = group_factory(1);

        let mut group1_with_two_addresses = group_factory(1);
        group1_with_two_addresses.addresses.push(USER2.to_string());

        // Instantiate with one group containing two addresses.
        let (mut app, contract_addr) =
            instantiate(Some(vec![group1_with_two_addresses.clone()])).unwrap();

        // Remove second address from group.
        app.execute_contract(
            Addr::unchecked(ADMIN),
            contract_addr.clone(),
            &ExecuteMsg::Remove {
                group: group1.name.clone(),
                addresses: Some(vec![USER2.to_string()]),
            },
            &[],
        )
        .unwrap();

        // Ensure there is one group with the expected contents.
        let dump_result = dump(&app, &contract_addr);
        assert_eq!(dump_result.groups.len(), 1);
        assert_eq!(dump_result.groups[0], group1);

        // Remove first address from group, emptying group.
        app.execute_contract(
            Addr::unchecked(ADMIN),
            contract_addr.clone(),
            &ExecuteMsg::Remove {
                group: group1.name,
                addresses: Some(group1.addresses),
            },
            &[],
        )
        .unwrap();

        // Ensure there are no groups.
        let dump_result = dump(&app, &contract_addr);
        assert_eq!(dump_result.groups.len(), 0);
    }

    #[test]
    fn remove_entire_group() {
        let mut group1_with_two_addresses = group_factory(1);
        group1_with_two_addresses.addresses.push(USER2.to_string());

        // Instantiate with one group containing two addresses.
        let (mut app, contract_addr) =
            instantiate(Some(vec![group1_with_two_addresses.clone()])).unwrap();

        // Remove entire group.
        app.execute_contract(
            Addr::unchecked(ADMIN),
            contract_addr.clone(),
            &ExecuteMsg::Remove {
                group: group1_with_two_addresses.name.clone(),
                addresses: None,
            },
            &[],
        )
        .unwrap();

        // Ensure there are no groups.
        let dump_result = dump(&app, &contract_addr);
        assert_eq!(dump_result.groups.len(), 0);
    }
}
