#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Order, Response, StdResult,
};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{
    DumpResponse, ExecuteMsg, Group, InstantiateMsg, IsAddressInGroupResponse,
    ListAddressesResponse, ListGroupsResponse, QueryMsg,
};

use crate::state::{GROUPS, OWNER};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:named-groups";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    OWNER.save(deps.storage, &info.sender)?;

    // Validate and save initial groups.
    if let Some(ref groups) = msg.groups {
        for group in groups {
            // Validate addresses.
            let addrs = group
                .addresses
                .iter()
                .map(|address| {
                    let addr = deps
                        .api
                        .addr_validate(address)
                        .map_err(|_| ContractError::InvalidAddress(address.clone()))?;
                    Ok(addr)
                })
                .collect::<Result<_, ContractError>>()?;

            GROUPS.update(deps.storage, &group.name, Some(addrs), None)?;
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
        ExecuteMsg::Update {
            group,
            addresses_to_add,
            addresses_to_remove,
        } => execute_update(deps, info, group, addresses_to_add, addresses_to_remove),
        ExecuteMsg::RemoveGroup { group } => execute_remove_group(deps, info, group),
        ExecuteMsg::UpdateOwner { owner } => execute_update_owner(deps, info, owner),
    }
}

fn execute_update(
    deps: DepsMut,
    info: MessageInfo,
    group: String,
    addresses_to_add: Option<Vec<String>>,
    addresses_to_remove: Option<Vec<String>>,
) -> Result<Response, ContractError> {
    let owner = OWNER.load(deps.storage)?;
    // Verify sender has permission.
    if info.sender != owner {
        return Err(ContractError::Unauthorized {});
    }

    let mut addrs_to_add: Option<Vec<Addr>> = None;
    // Validate addresses.
    if let Some(addrs) = &addresses_to_add {
        addrs_to_add = Some(
            addrs
                .iter()
                .map(|address| {
                    let addr = deps
                        .api
                        .addr_validate(address)
                        .map_err(|_| ContractError::InvalidAddress(address.clone()))?;
                    Ok(addr)
                })
                .collect::<Result<Vec<Addr>, ContractError>>()?,
        );
    }

    let mut addrs_to_remove: Option<Vec<Addr>> = None;
    // Validate addresses.
    if let Some(addrs) = &addresses_to_remove {
        addrs_to_remove = Some(
            addrs
                .iter()
                .map(|address| {
                    let addr = deps
                        .api
                        .addr_validate(address)
                        .map_err(|_| ContractError::InvalidAddress(address.clone()))?;
                    Ok(addr)
                })
                .collect::<Result<Vec<Addr>, ContractError>>()?,
        );
    }

    GROUPS.update(deps.storage, &group, addrs_to_add, addrs_to_remove)?;

    Ok(Response::default()
        .add_attribute("method", "add")
        .add_attribute("group", group)
        .add_attribute(
            "addresses_added",
            addresses_to_add.unwrap_or_default().len().to_string(),
        )
        .add_attribute(
            "addresses_removed",
            addresses_to_remove.unwrap_or_default().len().to_string(),
        ))
}

fn execute_remove_group(
    deps: DepsMut,
    info: MessageInfo,
    name: String,
) -> Result<Response, ContractError> {
    let owner = OWNER.load(deps.storage)?;
    // Verify sender has permission.
    if info.sender != owner {
        return Err(ContractError::Unauthorized {});
    }

    GROUPS.remove_group(deps.storage, &name)?;

    Ok(Response::default()
        .add_attribute("method", "remove")
        .add_attribute("group", name))
}

fn execute_update_owner(
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
    // Map groups to contained addresses.
    let mut dump: Vec<Group> = vec![];

    for group in GROUPS
        .group_names
        .keys(deps.storage, None, None, Order::Ascending)
        .into_iter()
    {
        if let Err(e) = group {
            return Err(e);
        }

        let name = group.unwrap();
        let addrs = GROUPS
            .groups_to_addresses
            .prefix(&name)
            .keys(deps.storage, None, None, Order::Ascending)
            .into_iter()
            .map(|addr| match addr {
                Ok(_) => Ok(addr.unwrap().to_string()),
                Err(_) => Err(addr.unwrap_err()),
            })
            .collect::<StdResult<Vec<String>>>()?;

        dump.push(Group {
            name,
            addresses: addrs,
        });
    }

    Ok(DumpResponse { groups: dump })
}

fn query_list_addresses(
    deps: Deps,
    group: String,
    offset: Option<u32>,
    limit: Option<u32>,
) -> StdResult<ListAddressesResponse> {
    let addresses = GROUPS.list_addresses(deps.storage, group)?;
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
    let groups = GROUPS.list_groups(deps.storage, &addr);

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
    let is_in_group = GROUPS.is_in_group(deps.storage, &addr, group)?;
    Ok(IsAddressInGroupResponse { is_in_group })
}
