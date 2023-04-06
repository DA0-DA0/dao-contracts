use cosmwasm_std::Empty;
use cosmwasm_std::{
    entry_point, from_binary, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Order, Response,
    StdResult, SubMsg, Uint64,
};
use cw4::{
    Member, MemberChangedHookMsg, MemberDiff, MemberListResponse, MemberResponse,
    TotalWeightResponse,
};
use cw721::{NftInfoResponse, OwnerOfResponse};
pub use cw721_base::{
    entry::{execute as _execute, query as _query},
    Cw721Contract, ExecuteMsg, InstantiateMsg as Cw721BaseInstantiateMsg, MinterResponse, QueryMsg,
};
use cw_storage_plus::Bound;
use cw_utils::maybe_addr;

use crate::msg::{ExecuteExt, MetadataExt, QueryExt};
use crate::state::{MEMBERS, TOTAL};
use crate::{error::RolesContractError as ContractError, state::HOOKS};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cw721-soulbound-roles";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub type Cw721NonTransferableContract<'a> =
    Cw721Contract<'a, MetadataExt, Empty, ExecuteExt, QueryExt>;

#[entry_point]
pub fn instantiate(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: Cw721BaseInstantiateMsg,
) -> Result<Response, ContractError> {
    Cw721NonTransferableContract::default().instantiate(deps.branch(), env.clone(), info, msg)?;

    // Initialize total weight to zero
    TOTAL.save(deps.storage, &0, env.block.height)?;

    cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::default()
        .add_attribute("contract_name", CONTRACT_NAME)
        .add_attribute("contract_version", CONTRACT_VERSION))
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg<MetadataExt, ExecuteExt>,
) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    match msg {
        ExecuteMsg::Mint {
            token_id,
            owner,
            token_uri,
            extension,
        } => execute_mint(deps, env, info, token_id, owner, token_uri, extension),
        ExecuteMsg::Burn { token_id } => execute_burn(deps, env, info, token_id),
        ExecuteMsg::Extension { msg } => match msg {
            ExecuteExt::AddHook { addr } => execute_add_hook(deps, info, addr),
            ExecuteExt::RemoveHook { addr } => execute_remove_hook(deps, info, addr),
        },
        // TODO send and transfer?
        // TODO approvals?
        _ => Cw721NonTransferableContract::default()
            .execute(deps, env, info, msg)
            .map_err(Into::into),
    }
}

pub fn execute_mint(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    token_id: String,
    owner: String,
    token_uri: Option<String>,
    extension: MetadataExt,
) -> Result<Response, ContractError> {
    let mut total = Uint64::from(TOTAL.load(deps.storage)?);
    let mut diff = MemberDiff::new(owner.clone(), None, None);

    // Update member weights and total
    MEMBERS.update(
        deps.storage,
        &deps.api.addr_validate(&owner)?,
        env.block.height,
        |old| -> StdResult<_> {
            // Increment the total weight by the weight of the new token
            total = total.checked_add(Uint64::from(extension.weight))?;
            // Add the new NFT weight to the old weight for the owner
            let new_weight = old.unwrap_or_default() + extension.weight;
            // Set the diff for use in hooks
            diff = MemberDiff::new(owner.clone(), old, Some(new_weight));
            Ok(new_weight)
        },
    )?;
    TOTAL.save(deps.storage, &total.u64(), env.block.height)?;

    let diffs = MemberChangedHookMsg { diffs: vec![diff] };

    // Prepare hook messages
    let msgs = HOOKS.prepare_hooks(deps.storage, |h| {
        diffs.clone().into_cosmos_msg(h).map(SubMsg::new)
    })?;

    // TODO call in a submessage? Add DAO to approvals? Or just implement the methods?
    // Call base mint
    let res = Cw721NonTransferableContract::default().execute(
        deps,
        env,
        info,
        ExecuteMsg::Mint {
            token_id,
            owner,
            token_uri,
            extension,
        },
    )?;

    Ok(res.add_submessages(msgs))
}

pub fn execute_burn(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    token_id: String,
) -> Result<Response, ContractError> {
    // Lookup the owner of the NFT
    let owner: OwnerOfResponse = from_binary(&Cw721NonTransferableContract::default().query(
        deps.as_ref(),
        env.clone(),
        QueryMsg::OwnerOf {
            token_id: token_id.clone(),
            include_expired: None,
        },
    )?)?;

    // Get the weight of the token
    let nft_info: NftInfoResponse<MetadataExt> =
        from_binary(&Cw721NonTransferableContract::default().query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::NftInfo {
                token_id: token_id.clone(),
            },
        )?)?;

    let mut total = Uint64::from(TOTAL.load(deps.storage)?);
    let mut diff = MemberDiff::new(owner.owner.clone(), None, None);

    // Update member weights and total
    let owner_addr = deps.api.addr_validate(&owner.owner)?;
    let old = MEMBERS.may_load(deps.storage, &owner_addr)?;

    // Only process this if they were actually in the list before
    if let Some(old_weight) = old {
        // Subtract the nft weight from the old weight
        //// TODO no unwrap
        let new_weight = old_weight.checked_sub(nft_info.extension.weight).unwrap();
        // Subtract nft weight from the total
        total = total.checked_sub(Uint64::from(nft_info.extension.weight))?;

        // Check if the new weight is now zero
        if new_weight == 0 {
            // New weight is now None
            diff = MemberDiff::new(owner.owner, Some(old_weight), None);
            // Owner owner from list of members
            MEMBERS.remove(deps.storage, &owner_addr, env.block.height)?;
        } else {
            MEMBERS.update(
                deps.storage,
                &owner_addr,
                env.block.height,
                |old| -> StdResult<_> {
                    diff = MemberDiff::new(owner.owner.clone(), old, Some(new_weight));
                    Ok(new_weight)
                },
            )?;
        }
    }

    TOTAL.save(deps.storage, &total.u64(), env.block.height)?;

    let diffs = MemberChangedHookMsg { diffs: vec![diff] };

    // Prepare hook messages
    let msgs = HOOKS.prepare_hooks(deps.storage, |h| {
        diffs.clone().into_cosmos_msg(h).map(SubMsg::new)
    })?;

    // Remove the token
    Cw721NonTransferableContract::default()
        .tokens
        .remove(deps.storage, &token_id)?;
    // Decrement the account
    Cw721NonTransferableContract::default().decrement_tokens(deps.storage)?;

    Ok(Response::new()
        .add_attribute("action", "burn")
        .add_attribute("sender", info.sender)
        .add_attribute("token_id", token_id)
        .add_submessages(msgs))
}

pub fn execute_add_hook(
    deps: DepsMut,
    info: MessageInfo,
    addr: String,
) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    let hook = deps.api.addr_validate(&addr)?;
    HOOKS.add_hook(deps.storage, hook)?;

    Ok(Response::default()
        .add_attribute("action", "add_hook")
        .add_attribute("hook", addr))
}

pub fn execute_remove_hook(
    deps: DepsMut,
    info: MessageInfo,
    addr: String,
) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    let hook = deps.api.addr_validate(&addr)?;
    HOOKS.remove_hook(deps.storage, hook)?;

    Ok(Response::default()
        .add_attribute("action", "remove_hook")
        .add_attribute("hook", addr))
}

#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg<QueryExt>) -> StdResult<Binary> {
    match msg {
        QueryMsg::Extension { msg } => match msg {
            QueryExt::Hooks {} => to_binary(&HOOKS.query_hooks(deps)?),
            QueryExt::Member { addr, at_height } => {
                to_binary(&query_member(deps, addr, at_height)?)
            }
            QueryExt::TotalWeight { at_height } => to_binary(&query_total_weight(deps, at_height)?),
        },
        _ => Cw721NonTransferableContract::default().query(deps, env, msg.into()),
    }
}

pub fn query_total_weight(deps: Deps, height: Option<u64>) -> StdResult<TotalWeightResponse> {
    let weight = match height {
        Some(h) => TOTAL.may_load_at_height(deps.storage, h),
        None => TOTAL.may_load(deps.storage),
    }?
    .unwrap_or_default();
    Ok(TotalWeightResponse { weight })
}

pub fn query_member(deps: Deps, addr: String, height: Option<u64>) -> StdResult<MemberResponse> {
    let addr = deps.api.addr_validate(&addr)?;
    let weight = match height {
        Some(h) => MEMBERS.may_load_at_height(deps.storage, &addr, h),
        None => MEMBERS.may_load(deps.storage, &addr),
    }?;
    Ok(MemberResponse { weight })
}

// settings for pagination
const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;

pub fn query_list_members(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<MemberListResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let addr = maybe_addr(deps.api, start_after)?;
    let start = addr.as_ref().map(Bound::exclusive);

    let members = MEMBERS
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            item.map(|(addr, weight)| Member {
                addr: addr.into(),
                weight,
            })
        })
        .collect::<StdResult<_>>()?;

    Ok(MemberListResponse { members })
}
