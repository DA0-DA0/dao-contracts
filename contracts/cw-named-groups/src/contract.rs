use std::collections::HashSet;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Order, Response, StdResult,
};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{DumpResponse, ExecuteMsg, Group, InstantiateMsg, QueryMsg};
use crate::state::{GROUPS, OWNER};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:named-groups";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

fn validate_addresses(
    deps: &DepsMut,
    addresses: impl IntoIterator<Item = String>,
) -> Result<HashSet<Addr>, ContractError> {
    let mut validated: HashSet<Addr> = HashSet::new();
    for address in addresses {
        let addr = deps
            .api
            .addr_validate(&address)
            .map_err(|_| ContractError::InvalidAddress(address.clone()))?;
        validated.insert(addr);
    }
    Ok(validated)
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
    if let Some(groups) = msg.groups.clone() {
        for group in groups {
            let addrs = validate_addresses(&deps, group.addresses)?;
            GROUPS.save(deps.storage, &group.name, &addrs)?;
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
    }
}

fn execute_add(
    deps: DepsMut,
    info: MessageInfo,
    group: String,
    addresses: Vec<String>,
) -> Result<Response, ContractError> {
    let owner = OWNER.load(deps.storage)?;
    // Verify sender has permission.
    if info.sender != owner {
        return Err(ContractError::Unauthorized {});
    }

    // Only attempt to add if addresses are provided.
    if !addresses.is_empty() {
        // Validate addresses.
        let addrs = validate_addresses(&deps, addresses)?;

        // Add addresses to group.
        GROUPS
            .update(deps.storage, &group, |existing_group| {
                let mut group = existing_group.unwrap_or_default();
                // Add new addresses.
                group.extend(addrs);

                Ok(group)
            })
            .map_err(ContractError::Std)?;
    }

    let addresses_in_group = GROUPS
        .load(deps.storage, &group)
        .unwrap_or_default()
        .len()
        .to_string();

    Ok(Response::default()
        .add_attribute("method", "add")
        .add_attribute("group", group)
        .add_attribute("addresses_in_group", addresses_in_group))
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
    let mut group_addresses = GROUPS
        .load(deps.storage, &group)
        .map_err(|_| ContractError::InvalidGroup(group.clone()))?;

    // If provided addresses, remove them from the group, removing the group if it becomes empty.
    if let Some(addresses) = addresses {
        // Only attempt to remove if addresses are provided.
        if !addresses.is_empty() {
            // Validate addresses.
            let addrs = validate_addresses(&deps, addresses)?;

            // Remove addresses from group.
            addrs.iter().for_each(|addr| {
                group_addresses.remove(addr);
            });
            // Remove group entirely if empty.
            if group_addresses.is_empty() {
                GROUPS.remove(deps.storage, &group);
            } else {
                GROUPS.save(deps.storage, &group, &group_addresses)?;
            }
        }
    }
    // Otherwise remove the entire group.
    else {
        GROUPS.remove(deps.storage, &group);
    }

    Ok(Response::default()
        .add_attribute("method", "remove")
        .add_attribute("group", group)
        .add_attribute("addresses_in_group", group_addresses.len().to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Dump {} => to_binary(&query_dump(deps)?),
    }
}

fn query_dump(deps: Deps) -> StdResult<DumpResponse> {
    let mut dump: Vec<Group> = vec![];

    // Load groups into dump.
    let groups = GROUPS.keys(deps.storage, None, None, Order::Ascending);
    for group in groups.flatten() {
        let addresses = GROUPS.load(deps.storage, &group)?;
        dump.push(Group {
            name: group,
            addresses: addresses.into_iter().map(|addr| addr.to_string()).collect(),
        });
    }

    Ok(DumpResponse { groups: dump })
}
