use crate::{
    msg::{
        DumpResponse, ExecuteMsg, Group, InstantiateMsg, ListAddressesResponse, ListGroupsResponse,
        QueryMsg,
    },
    ContractError,
};
use cosmwasm_std::{Addr, Empty, StdError, StdResult};
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
const USER3: &str = "USER3";

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

fn list_addresses(
    app: &App,
    contract_addr: &Addr,
    group: String,
    offset: Option<usize>,
    limit: Option<usize>,
) -> StdResult<ListAddressesResponse> {
    app.wrap().query_wasm_smart(
        contract_addr,
        &QueryMsg::ListAddresses {
            group,
            offset,
            limit,
        },
    )
}

fn list_groups(
    app: &App,
    contract_addr: &Addr,
    address: String,
    offset: Option<usize>,
    limit: Option<usize>,
) -> StdResult<ListGroupsResponse> {
    app.wrap().query_wasm_smart(
        contract_addr,
        &QueryMsg::ListGroups {
            address,
            offset,
            limit,
        },
    )
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
        let group1 = group_factory(1);
        let groups = vec![group1.clone(), group_factory(2), group_factory(3)];

        let (app, contract_addr) = instantiate(Some(groups.clone())).unwrap();

        // Ensure there are the expected groups in dump.
        let dump_result = dump(&app, &contract_addr);
        assert_eq!(dump_result.groups, groups);

        // Ensure there are the expected addresses for a group.
        let addresses = list_addresses(&app, &contract_addr, group1.name.clone(), None, None)
            .unwrap()
            .addresses;
        assert_eq!(addresses, group1.addresses);

        // Ensure there are the expected groups for an address.
        let groups = list_groups(
            &app,
            &contract_addr,
            group1.addresses.get(0).unwrap().clone(),
            None,
            None,
        )
        .unwrap()
        .groups;
        assert_eq!(groups, vec![group1.name]);
    }
}

mod add {
    use std::collections::HashSet;

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
                    addresses: Some(vec![USER1.to_string()]),
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
                addresses: Some(group1.addresses.to_vec()),
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
                addresses: Some(addresses[..1].to_vec()),
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
                addresses: Some(addresses[1..].to_vec()),
            },
            &[],
        )
        .unwrap();

        // Ensure there is one group with the expected contents.
        let dump_result = dump(&app, &contract_addr);
        assert_eq!(dump_result.groups.len(), 1);
        assert_eq!(dump_result.groups[0].name, group_name);
        assert_eq!(
            dump_result.groups[0]
                .addresses
                .clone()
                .into_iter()
                .collect::<HashSet<String>>(),
            addresses.into_iter().collect::<HashSet<String>>()
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
                addresses: Some(group1.addresses.to_vec()),
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
                addresses: Some(group2.addresses.to_vec()),
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
                    addresses: Some(group1.addresses.to_vec()),
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
                    addresses: Some(group1.addresses.to_vec()),
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
                addresses: Some(group1.addresses.to_vec()),
            },
            &[],
        )
        .unwrap();

        // Ensure there is still 1 group but it is empty.
        let dump_result = dump(&app, &contract_addr);
        assert_eq!(dump_result.groups.len(), 1);
        assert_eq!(dump_result.groups[0].addresses.len(), 0);
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

mod change_owner {
    use super::*;

    #[test]
    fn change_owner_unauthorized() {
        let (mut app, contract_addr) = instantiate(None).unwrap();

        // Try to change the owner.
        let err: ContractError = app
            .execute_contract(
                // not the owner
                Addr::unchecked(USER1),
                contract_addr.clone(),
                &ExecuteMsg::ChangeOwner {
                    owner: USER1.to_string(),
                },
                &[],
            )
            .unwrap_err()
            .downcast()
            .unwrap();
        // Expect unauthorized.
        assert_eq!(err, ContractError::Unauthorized {});

        // Ensure the attempted new owner cannot do anything by
        // trying to add a new group and expecting failure.
        let group1 = group_factory(1);
        let err: ContractError = app
            .execute_contract(
                // not the owner
                Addr::unchecked(USER1),
                contract_addr,
                &ExecuteMsg::Add {
                    group: group1.name.clone(),
                    addresses: Some(group1.addresses.to_vec()),
                },
                &[],
            )
            .unwrap_err()
            .downcast()
            .unwrap();
        // Expect unauthorized.
        assert_eq!(err, ContractError::Unauthorized {});
    }

    #[test]
    fn change_owner_success() {
        let (mut app, contract_addr) = instantiate(None).unwrap();

        // Change the owner.
        app.execute_contract(
            // the current owner
            Addr::unchecked(ADMIN),
            contract_addr.clone(),
            &ExecuteMsg::ChangeOwner {
                owner: USER1.to_string(),
            },
            &[],
        )
        .unwrap();

        let group1 = group_factory(1);

        // Ensure the old owner cannot do anything by
        // trying to add a new group and expecting failure.
        let err: ContractError = app
            .execute_contract(
                // the old owner
                Addr::unchecked(ADMIN),
                contract_addr.clone(),
                &ExecuteMsg::Add {
                    group: group1.name.clone(),
                    addresses: Some(group1.addresses.to_vec()),
                },
                &[],
            )
            .unwrap_err()
            .downcast()
            .unwrap();
        // Expect unauthorized.
        assert_eq!(err, ContractError::Unauthorized {});

        // Ensure the new owner is authorized.
        app.execute_contract(
            // the new owner
            Addr::unchecked(USER1),
            contract_addr.clone(),
            &ExecuteMsg::Add {
                group: group1.name.clone(),
                addresses: Some(group1.addresses.to_vec()),
            },
            &[],
        )
        .unwrap();
        // Ensure there is one group with the expected contents.
        let dump_result = dump(&app, &contract_addr);
        assert_eq!(dump_result.groups.len(), 1);
        assert_eq!(dump_result.groups[0], group1);
    }
}

mod list_addresses {
    use super::*;

    #[test]
    fn group_not_found() {
        let (app, contract_addr) = instantiate(None).unwrap();

        let group1 = group_factory(1);

        // Try to list addresses from a non-existent group.
        let err = list_addresses(&app, &contract_addr, group1.name, None, None).unwrap_err();

        // Expect group not found.
        // Not sure why this becomes a generic error and not the StdError::NotFound enum but whatever.
        assert_eq!(
            err,
            StdError::generic_err("Querier contract error: group not found")
        );
    }

    #[test]
    fn empty_group() {
        let (mut app, contract_addr) = instantiate(None).unwrap();

        let group1 = group_factory(1);
        // Add empty group.
        app.execute_contract(
            Addr::unchecked(ADMIN),
            contract_addr.clone(),
            &ExecuteMsg::Add {
                group: group1.name.clone(),
                addresses: None,
            },
            &[],
        )
        .unwrap();

        // List addresses from the group.
        let addresses = list_addresses(&app, &contract_addr, group1.name, None, None)
            .unwrap()
            .addresses;

        // Expect empty group.
        assert_eq!(addresses, Vec::<Addr>::new());
    }

    #[test]
    fn populated_group() {
        let (mut app, contract_addr) = instantiate(None).unwrap();

        let group1 = group_factory(1);
        // Add group.
        app.execute_contract(
            Addr::unchecked(ADMIN),
            contract_addr.clone(),
            &ExecuteMsg::Add {
                group: group1.name.clone(),
                addresses: Some(group1.addresses.to_vec()),
            },
            &[],
        )
        .unwrap();

        // List addresses from the group.
        let addresses = list_addresses(&app, &contract_addr, group1.name, None, None)
            .unwrap()
            .addresses;

        // Expect group addresses.
        assert_eq!(addresses, group1.addresses.to_vec());
    }

    #[test]
    fn paginate_populated_group() {
        let mut group1 = group_factory(1);
        group1
            .addresses
            .extend(vec![USER2.to_string(), USER3.to_string()]);

        let (app, contract_addr) = instantiate(Some(vec![group1.clone()])).unwrap();

        // No pagination.
        let addresses = list_addresses(&app, &contract_addr, group1.name.clone(), None, None)
            .unwrap()
            .addresses;
        // Expect all addresses.
        assert_eq!(addresses.len(), group1.addresses.len());

        // Pagination, get all.
        let addresses = list_addresses(
            &app,
            &contract_addr,
            group1.name.clone(),
            Some(0),
            Some(group1.addresses.len()),
        )
        .unwrap()
        .addresses;
        // Expect all addresses.
        assert_eq!(addresses.len(), group1.addresses.len());

        // Pagination, get 1.
        let addresses = list_addresses(&app, &contract_addr, group1.name.clone(), Some(1), Some(1))
            .unwrap()
            .addresses;
        // Expect 1 address.
        assert_eq!(addresses.len(), 1);

        // Pagination, get 2.
        let addresses = list_addresses(&app, &contract_addr, group1.name.clone(), None, Some(2))
            .unwrap()
            .addresses;
        // Expect 2 addresses.
        assert_eq!(addresses.len(), 2);

        // Pagination, get 2.
        let addresses = list_addresses(&app, &contract_addr, group1.name.clone(), Some(1), Some(2))
            .unwrap()
            .addresses;
        // Expect 2 addresses.
        assert_eq!(addresses.len(), 2);

        // Pagination, high limit, get 1 since offset is 1 from end.
        let addresses =
            list_addresses(&app, &contract_addr, group1.name.clone(), Some(2), Some(10))
                .unwrap()
                .addresses;
        // Expect 1 address.
        assert_eq!(addresses.len(), 1);

        // Pagination, get none when limit is 0.
        let addresses = list_addresses(&app, &contract_addr, group1.name, Some(1), Some(0))
            .unwrap()
            .addresses;
        // Expect 0 addresses.
        assert_eq!(addresses.len(), 0);
    }
}

mod list_groups {
    use super::*;

    #[test]
    fn address_not_found() {
        let (app, contract_addr) = instantiate(None).unwrap();

        // List groups from a non-existent address.
        let groups = list_groups(&app, &contract_addr, "ADDRESS".to_string(), None, None)
            .unwrap()
            .groups;

        // Expect empty list
        assert_eq!(groups, Vec::<String>::new());
    }

    #[test]
    fn empty_group() {
        let (mut app, contract_addr) = instantiate(None).unwrap();

        let group1 = group_factory(1);
        // Add empty group.
        app.execute_contract(
            Addr::unchecked(ADMIN),
            contract_addr.clone(),
            &ExecuteMsg::Add {
                group: group1.name,
                addresses: None,
            },
            &[],
        )
        .unwrap();

        // List groups from a non-existent address.
        let groups = list_groups(&app, &contract_addr, "ADDRESS".to_string(), None, None)
            .unwrap()
            .groups;

        // Expect empty list
        assert_eq!(groups, Vec::<String>::new());
    }

    #[test]
    fn populated_group() {
        let (mut app, contract_addr) = instantiate(None).unwrap();

        let group1 = group_factory(1);
        // Add group.
        app.execute_contract(
            Addr::unchecked(ADMIN),
            contract_addr.clone(),
            &ExecuteMsg::Add {
                group: group1.name.clone(),
                addresses: Some(group1.addresses.to_vec()),
            },
            &[],
        )
        .unwrap();

        // List groups for address in group.
        let groups = list_groups(
            &app,
            &contract_addr,
            group1.addresses.get(0).unwrap().to_string(),
            None,
            None,
        )
        .unwrap()
        .groups;

        // Expect address groups to be the added group.
        assert_eq!(groups, vec![group1.name]);
    }

    #[test]
    fn paginate_populated_group() {
        // Three groups with the same address in each one.
        let group1 = group_factory(1);
        let mut group2 = group_factory(2);
        let mut group3 = group_factory(3);
        group2.addresses = group1.addresses.clone();
        group3.addresses = group1.addresses.clone();

        let all_groups = vec![group1.clone(), group2, group3];
        let groups_count = all_groups.len();
        let address = group1.addresses.get(0).unwrap();

        let (app, contract_addr) = instantiate(Some(all_groups)).unwrap();

        // No pagination.
        let groups = list_groups(&app, &contract_addr, address.clone(), None, None)
            .unwrap()
            .groups;
        // Expect all groups.
        assert_eq!(groups.len(), groups_count);

        // Pagination, get all.
        let groups = list_groups(
            &app,
            &contract_addr,
            address.clone(),
            Some(0),
            Some(groups_count),
        )
        .unwrap()
        .groups;
        // Expect all groups.
        assert_eq!(groups.len(), groups_count);

        // Pagination, get 1.
        let groups = list_groups(&app, &contract_addr, address.clone(), Some(1), Some(1))
            .unwrap()
            .groups;
        // Expect 1 group.
        assert_eq!(groups.len(), 1);

        // Pagination, get 2.
        let groups = list_groups(&app, &contract_addr, address.clone(), None, Some(2))
            .unwrap()
            .groups;
        // Expect 2 groups.
        assert_eq!(groups.len(), 2);

        // Pagination, get 2.
        let groups = list_groups(&app, &contract_addr, address.clone(), Some(1), Some(2))
            .unwrap()
            .groups;
        // Expect 2 groups.
        assert_eq!(groups.len(), 2);

        // Pagination, high limit, get 1 since offset is 1 from end.
        let groups = list_groups(&app, &contract_addr, address.clone(), Some(2), Some(1000))
            .unwrap()
            .groups;
        // Expect 1 group.
        assert_eq!(groups.len(), 1);

        // Pagination, get none when limit is 0.
        let groups = list_groups(&app, &contract_addr, address.clone(), Some(1), Some(0))
            .unwrap()
            .groups;
        // Expect 0 groups.
        assert_eq!(groups.len(), 0);
    }
}

mod is_address_in_group {
    use super::*;
    use crate::msg::IsAddressInGroupResponse;

    fn is_address_in_group(
        app: &App,
        contract_addr: &Addr,
        address: String,
        group: String,
    ) -> StdResult<IsAddressInGroupResponse> {
        app.wrap().query_wasm_smart(
            contract_addr,
            &QueryMsg::IsAddressInGroup { address, group },
        )
    }

    #[test]
    fn group_not_found() {
        let (app, contract_addr) = instantiate(None).unwrap();

        // Check if address is in non-existent group.
        let err = is_address_in_group(
            &app,
            &contract_addr,
            "ADDRESS".to_string(),
            "GROUP".to_string(),
        )
        .unwrap_err();

        // Expect group not found.
        // Not sure why this becomes a generic error and not the StdError::NotFound enum but whatever.
        assert_eq!(
            err,
            StdError::generic_err("Querier contract error: group not found")
        );
    }

    #[test]
    fn empty_group() {
        let (mut app, contract_addr) = instantiate(None).unwrap();

        let group1 = group_factory(1);
        // Add empty group.
        app.execute_contract(
            Addr::unchecked(ADMIN),
            contract_addr.clone(),
            &ExecuteMsg::Add {
                group: group1.name.clone(),
                addresses: None,
            },
            &[],
        )
        .unwrap();

        // Check if address is in group.
        let is_in_group =
            is_address_in_group(&app, &contract_addr, "ADDRESS".to_string(), group1.name)
                .unwrap()
                .is_in_group;

        // Expect not in group.
        assert!(!is_in_group);
    }

    #[test]
    fn populated_group() {
        let (mut app, contract_addr) = instantiate(None).unwrap();

        let group1 = group_factory(1);
        // Add group.
        app.execute_contract(
            Addr::unchecked(ADMIN),
            contract_addr.clone(),
            &ExecuteMsg::Add {
                group: group1.name.clone(),
                addresses: Some(group1.addresses.to_vec()),
            },
            &[],
        )
        .unwrap();

        // Check if address is in group.
        let is_in_group = is_address_in_group(
            &app,
            &contract_addr,
            group1.addresses.get(0).unwrap().to_string(),
            group1.name,
        )
        .unwrap()
        .is_in_group;

        // Expect in group.
        assert!(is_in_group);
    }
}
