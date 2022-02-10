use std::collections::BTreeSet;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{AddressItem, ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{ADMIN, ITEMS};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:string-manager";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let admin = deps.api.addr_validate(msg.admin.as_str())?;

    ADMIN.save(deps.storage, &admin)?;
    ITEMS.save(deps.storage, &BTreeSet::default())?;

    Ok(Response::default()
        .add_attribute("method", "instantiate")
        .add_attribute("sender", info.sender)
        .add_attribute("admin", admin))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    // Admin is the only address that can change this contract's
    // state.
    let admin = ADMIN.load(deps.storage)?;
    if info.sender != admin {
        return Err(ContractError::Unauthorized {});
    }

    match msg {
        ExecuteMsg::UpdateAddresses { to_add, to_remove } => {
            execute_update_addresses(deps, to_add, to_remove)
        }
        ExecuteMsg::UpdateAdmin { new_admin } => execute_update_admin(deps, new_admin),
    }
}

pub fn execute_update_addresses(
    deps: DepsMut,
    to_add: Vec<AddressItem>,
    to_remove: Vec<AddressItem>,
) -> Result<Response, ContractError> {
    let mut items = ITEMS.load(deps.storage)?;

    // O(n * log(N))
    items.extend(to_add.into_iter());

    // O(n * log(N))
    for item in to_remove.into_iter() {
        items.remove(&item);
    }

    ITEMS.save(deps.storage, &items)?;

    Ok(Response::default().add_attribute("method", "update_addresses"))
}

pub fn execute_update_admin(deps: DepsMut, new_admin: Addr) -> Result<Response, ContractError> {
    let admin = deps.api.addr_validate(new_admin.as_str())?;
    ADMIN.save(deps.storage, &admin)?;
    Ok(Response::default()
        .add_attribute("method", "update_admin")
        .add_attribute("new_admin", admin))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetAddresses {} => query_get_addresses(deps),
        QueryMsg::GetAdmin {} => query_get_admin(deps),
        QueryMsg::GetAddressCount {} => query_get_address_count(deps),
    }
}

pub fn query_get_admin(deps: Deps) -> StdResult<Binary> {
    let admin = ADMIN.load(deps.storage)?;
    to_binary(&admin)
}

pub fn query_get_addresses(deps: Deps) -> StdResult<Binary> {
    let items = ITEMS.load(deps.storage)?;
    let items: Vec<_> = items.into_iter().rev().collect();
    to_binary(&items)
}

pub fn query_get_address_count(deps: Deps) -> StdResult<Binary> {
    let items = ITEMS.load(deps.storage)?;
    to_binary(&items.len())
}
