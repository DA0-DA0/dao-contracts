use std::collections::HashSet;
use std::hash::Hash;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Api, Binary, Deps, DepsMut, Env, MessageInfo, Order, Response, StdError,
    StdResult, Storage,
};
use cw2::set_contract_version;
use cw_storage_plus::{Map, PrimaryKey};
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::error::ContractError;
use crate::msg::{
    DumpResponse, ExecuteMsg, Group, InstantiateMsg, IsAddressInGroupResponse,
    ListAddressesResponse, ListGroupsResponse, QueryMsg,
};
use crate::state::{ADDRESSES, GROUPS, OWNER};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:named-groups";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

fn validate_addresses(
    api: &dyn Api,
    addresses: impl IntoIterator<Item = String>,
) -> Result<HashSet<Addr>, ContractError> {
    let addrs = addresses
        .into_iter()
        .map(|address| {
            let addr = api
                .addr_validate(&address)
                .map_err(|_| ContractError::InvalidAddress(address.clone()))?;
            Ok(addr)
        })
        .collect::<Result<HashSet<Addr>, ContractError>>()?;
    Ok(addrs)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Save owner.
    OWNER.save(deps.storage, &info.sender)?;

    // Validate and save initial groups.
    if let Some(ref groups) = msg.groups {
        for group in groups {
            let addrs = validate_addresses(deps.api, group.addresses.clone())?;
            GROUPS.save(deps.storage, &group.name, &addrs)?;

            // Add group to each address's group list.
            for addr in addrs.iter() {
                add_to_map(
                    deps.storage,
                    ADDRESSES,
                    addr.clone(),
                    vec![group.name.clone()],
                )?;
            }
        }
    }

    Ok(Response::default()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender.to_string())
        .add_attribute("groups", msg.groups.unwrap_or_default().len().to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Add { group, addresses } => execute_add(deps, info, group, addresses),
        ExecuteMsg::Remove { group, addresses } => execute_remove(deps, info, group, addresses),
        ExecuteMsg::ChangeOwner { owner } => execute_change_owner(deps, info, owner),
    }
}

fn execute_add(
    deps: DepsMut,
    info: MessageInfo,
    group: String,
    addresses: Option<Vec<String>>,
) -> Result<Response, ContractError> {
    let owner = OWNER.load(deps.storage)?;
    // Verify sender has permission.
    if info.sender != owner {
        return Err(ContractError::Unauthorized {});
    }

    // If provided addresses, add them to the group.
    if let Some(ref addresses) = addresses {
        // Only attempt to add if addresses are provided.
        if !addresses.is_empty() {
            // Validate addresses.
            let addrs = validate_addresses(deps.api, addresses.clone())?;

            // Add group to each address's group list.
            for addr in addrs.iter() {
                add_to_map(deps.storage, ADDRESSES, addr.clone(), vec![group.clone()])?;
            }

            // Add addresses to group map.
            add_to_map(deps.storage, GROUPS, &group, addrs)?;
        }
    }
    // Otherwise add an empty group.
    else {
        GROUPS.save(deps.storage, &group, &HashSet::new())?;
    }

    let addresses_added = addresses.map(|a| a.len()).unwrap_or_default();

    Ok(Response::default()
        .add_attribute("method", "add")
        .add_attribute("group", group)
        .add_attribute("addresses_added", addresses_added.to_string()))
}

fn execute_remove(
    deps: DepsMut,
    info: MessageInfo,
    group: String,
    addresses: Option<Vec<String>>,
) -> Result<Response, ContractError> {
    let owner = OWNER.load(deps.storage)?;
    // Verify sender has permission.
    if info.sender != owner {
        return Err(ContractError::Unauthorized {});
    }

    // Verify group exists.
    GROUPS
        .load(deps.storage, &group)
        .map_err(|_| ContractError::InvalidGroup(group.clone()))?;

    // If provided addresses, remove them from the group.
    if let Some(ref addresses) = addresses {
        // Only attempt to remove if addresses are provided.
        if !addresses.is_empty() {
            // Validate addresses.
            let addrs = validate_addresses(deps.api, addresses.clone())?;

            // Remove group from each address's group list.
            let group_set_to_remove = vec![group.clone()];
            for addr in addrs.iter() {
                remove_from_map(
                    deps.storage,
                    ADDRESSES,
                    addr.clone(),
                    group_set_to_remove.iter(),
                )?;
            }

            // Remove addresses from group map.
            remove_from_map(deps.storage, GROUPS, &group, addrs.iter())?;
        }
    }
    // Otherwise remove the group.
    else {
        GROUPS.remove(deps.storage, &group);
    }

    let addresses_removed = addresses.map(|a| a.len()).unwrap_or_default();

    Ok(Response::default()
        .add_attribute("method", "remove")
        .add_attribute("group", group)
        .add_attribute("addresses_removed", addresses_removed.to_string()))
}

fn execute_change_owner(
    deps: DepsMut,
    info: MessageInfo,
    new_owner: String,
) -> Result<Response, ContractError> {
    let curr_owner = OWNER.load(deps.storage)?;
    // Verify sender has permission.
    if info.sender != curr_owner {
        return Err(ContractError::Unauthorized {});
    }

    let new_owner = deps.api.addr_validate(&new_owner)?;
    OWNER.save(deps.storage, &new_owner)?;

    Ok(Response::default()
        .add_attribute("method", "change_owner")
        .add_attribute("owner", new_owner))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Dump {} => to_binary(&query_dump(deps)?),
        QueryMsg::ListAddresses {
            group,
            offset,
            limit,
        } => to_binary(&query_list_addresses(deps, group, offset, limit)?),
        QueryMsg::ListGroups {
            address,
            offset,
            limit,
        } => to_binary(&query_list_groups(deps, address, offset, limit)?),
        QueryMsg::IsAddressInGroup { address, group } => {
            to_binary(&query_is_address_in_group(deps, address, group)?)
        }
    }
}

fn query_dump(deps: Deps) -> StdResult<DumpResponse> {
    // Create dump of all groups.
    let dump = GROUPS
        .keys(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<String>>>()?
        .into_iter()
        .map(|group| {
            let addresses = GROUPS.load(deps.storage, &group)?;
            Ok(Group {
                name: group,
                addresses: addresses.into_iter().map(|addr| addr.to_string()).collect(),
            })
        })
        .collect::<StdResult<Vec<Group>>>()?;

    Ok(DumpResponse { groups: dump })
}

fn query_list_addresses(
    deps: Deps,
    group: String,
    offset: Option<u32>,
    limit: Option<u32>,
) -> StdResult<ListAddressesResponse> {
    let addresses = GROUPS
        .load(deps.storage, &group)
        .map_err(|_| StdError::not_found("group"))?;

    // Paginate.
    let default_take_all = addresses.len() as u32;
    let addresses = addresses
        .into_iter()
        .skip(offset.unwrap_or_default() as usize)
        .take(limit.unwrap_or(default_take_all) as usize)
        .collect();

    Ok(ListAddressesResponse { addresses })
}

fn query_list_groups(
    deps: Deps,
    address: String,
    offset: Option<u32>,
    limit: Option<u32>,
) -> StdResult<ListGroupsResponse> {
    // Validate address.
    let addr = deps.api.addr_validate(&address)?;
    // Return groups, or an empty set if failed to load (address probably doesn't exist).
    // It doesn't make sense to ask for the addresses in a group if the group doesn't exist, which is why
    // we return an error in query_list_addresses; however, here in query_list_groups, it makes sense
    // to return an empty list when an address is not in any groups since conceptually the structure
    // is One Group to Many Addresses.
    let groups = ADDRESSES.load(deps.storage, addr).unwrap_or_default();

    // Paginate.
    let default_take_all = groups.len() as u32;
    let groups = groups
        .into_iter()
        .skip(offset.unwrap_or_default() as usize)
        .take(limit.unwrap_or(default_take_all) as usize)
        .collect();

    Ok(ListGroupsResponse { groups })
}

fn query_is_address_in_group(
    deps: Deps,
    address: String,
    group: String,
) -> StdResult<IsAddressInGroupResponse> {
    // Validate address.
    let addr = deps.api.addr_validate(&address)?;
    // Verify group exists and get addresses for group.
    let addrs = GROUPS
        .load(deps.storage, &group)
        .map_err(|_| StdError::not_found("group"))?;

    Ok(IsAddressInGroupResponse {
        is_in_group: addrs.contains(&addr),
    })
}

fn add_to_map<'a, K, V>(
    storage: &mut dyn Storage,
    map: Map<'a, K, HashSet<V>>,
    key: K,
    values: impl IntoIterator<Item = V>,
) -> Result<(), ContractError>
where
    HashSet<V>: DeserializeOwned + Serialize,
    K: Eq + Hash + PrimaryKey<'a>,
    V: Eq + Hash,
{
    map.update(storage, key, |existing_val| {
        let mut set = existing_val.unwrap_or_default();
        set.extend(values);
        Ok(set)
    })
    .map_err(ContractError::Std)?;

    Ok(())
}

fn remove_from_map<'a, 'b, K, V>(
    storage: &mut dyn Storage,
    map: Map<'a, K, HashSet<V>>,
    key: K,
    values: impl Iterator<Item = &'b V>,
) -> Result<(), ContractError>
where
    HashSet<V>: DeserializeOwned + Serialize,
    K: Eq + Hash + PrimaryKey<'a>,
    V: 'b + Eq + Hash,
{
    map.update(storage, key, |existing_val| {
        let mut set = existing_val.unwrap_or_default();
        for value in values {
            set.remove(value);
        }
        Ok(set)
    })
    .map_err(ContractError::Std)?;

    Ok(())
}
