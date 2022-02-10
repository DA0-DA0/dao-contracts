#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, SubMsg,
};
use cw2::set_contract_version;
use cw4::{MemberChangedHookMsg, MemberResponse, TotalWeightResponse};

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::query::{Member, MemberListResponse};
use crate::state::{
    initialize_members, list_members_sorted, remove_member, update_member, ADMIN, HOOKS, MEMBERS,
    TOTAL,
};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:string-manager";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let admin = msg
        .admin
        .map(|admin| deps.api.addr_validate(&admin))
        .transpose()?;

    ADMIN.set(deps.branch(), admin)?;

    initialize_members(deps.branch(), msg.members, env.block.height)?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let api = deps.api;
    match msg {
        ExecuteMsg::UpdateAdmin { admin } => execute_update_admin(deps, admin, info),
        ExecuteMsg::UpdateMembers { remove, add } => {
            execute_update_members(deps, info, add, remove, env.block.height)
        }
        ExecuteMsg::AddHook { addr } => {
            Ok(HOOKS.execute_add_hook(&ADMIN, deps, info, api.addr_validate(&addr)?)?)
        }
        ExecuteMsg::RemoveHook { addr } => {
            Ok(HOOKS.execute_remove_hook(&ADMIN, deps, info, api.addr_validate(&addr)?)?)
        }
    }
}

pub fn execute_update_admin(
    deps: DepsMut,
    admin: Option<String>,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let api = deps.api;
    // `execute_update_admin` handles verifying that the sender is the
    // current admin.
    Ok(ADMIN.execute_update_admin(
        deps,
        info,
        admin.map(|admin| api.addr_validate(&admin)).transpose()?,
    )?)
}

pub fn execute_update_members(
    deps: DepsMut,
    info: MessageInfo,
    add: Vec<Member>,
    remove: Vec<String>,
    height: u64,
) -> Result<Response, ContractError> {
    ADMIN.assert_admin(deps.as_ref(), &info.sender)?;

    let attributes = vec![
        attr("action", "update_members"),
        attr("added", add.len().to_string()),
        attr("removed", remove.len().to_string()),
        attr("sender", &info.sender),
    ];

    let mut diffs = vec![];

    for member in add.into_iter() {
        let Member { addr, weight } = member;
        let addr = deps.api.addr_validate(addr.as_str())?;
        let diff = update_member(deps.storage, addr, weight, height)?;
        diffs.push(diff);
    }

    for addr in remove.into_iter() {
        let addr = deps.api.addr_validate(&addr)?;
        let diff = remove_member(deps.storage, addr, height)?;
        diffs.push(diff)
    }

    let diff = MemberChangedHookMsg { diffs };
    let messages = HOOKS.prepare_hooks(deps.storage, |h| {
        diff.clone().into_cosmos_msg(h).map(SubMsg::new)
    })?;

    Ok(Response::default()
        .add_submessages(messages)
        .add_attributes(attributes))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Admin {} => query_admin(deps),
        QueryMsg::TotalWeight {} => todo!(),
        QueryMsg::ListMembers { start_after, limit } => {
            query_list_members(deps, start_after, limit)
        }
        QueryMsg::Member { addr, at_height } => query_member(deps, addr, at_height),
        QueryMsg::Hooks {} => query_total_weight(deps),
    }
}

pub fn query_admin(deps: Deps) -> StdResult<Binary> {
    to_binary(&ADMIN.query_admin(deps)?)
}

pub fn query_total_weight(deps: Deps) -> StdResult<Binary> {
    let weight = TOTAL.load(deps.storage)?;
    to_binary(&TotalWeightResponse { weight })
}

pub fn query_member(deps: Deps, addr: String, height: Option<u64>) -> StdResult<Binary> {
    let addr = deps.api.addr_validate(&addr)?;
    let weight = match height {
        Some(h) => MEMBERS.may_load_at_height(deps.storage, &addr, h),
        None => MEMBERS.may_load(deps.storage, &addr),
    }?;
    to_binary(&MemberResponse { weight })
}

// This query is not gas efficent. When does it run out of gas? On the
// testnet somewhere around 5991 items in the address list. Happily,
// the limit happens due to the sheer amount of data returned by the
// query, not due to the compute used by the query. One can still
// query the contract's state by setting a limit on the amount of data
// returned and remove items as needed.
pub fn query_list_members(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Binary> {
    let start_after = start_after
        .map(|addr| deps.api.addr_validate(&addr))
        .transpose()?;

    let members = list_members_sorted(deps.storage)?;

    let matches = match start_after {
        Some(start_after) => {
            let mut seen = false;
            let (_before, after): (Vec<Member>, Vec<Member>) =
                members.into_iter().partition(|Member { addr, weight: _ }| {
                    if *addr == start_after {
                        seen = true
                    }
                    seen
                });
            after
        }
        None => members,
    };

    let matches: Vec<Member> = match limit {
        Some(limit) => matches.into_iter().take(limit as usize).collect(),
        None => matches,
    };

    to_binary(&MemberListResponse { members: matches })
}
