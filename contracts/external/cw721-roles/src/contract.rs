#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_json, to_json_binary, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Order, Response,
    StdResult, SubMsg, Uint64,
};
use cw4::{
    Member, MemberChangedHookMsg, MemberDiff, MemberListResponse, MemberResponse,
    TotalWeightResponse,
};
use cw721::{Cw721ReceiveMsg, NftInfoResponse, OwnerOfResponse};
use cw721_base::{Cw721Contract, InstantiateMsg as Cw721BaseInstantiateMsg};
use cw_storage_plus::Bound;
use cw_utils::maybe_addr;
use dao_cw721_extensions::roles::{ExecuteExt, MetadataExt, QueryExt};
use std::cmp::Ordering;

use crate::msg::{ExecuteMsg, QueryMsg};
use crate::state::{MEMBERS, TOTAL};
use crate::{error::RolesContractError as ContractError, state::HOOKS};

// Version info for migration
const CONTRACT_NAME: &str = "crates.io:cw721-roles";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

// Settings for query pagination
const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;

pub type Cw721Roles<'a> = Cw721Contract<'a, MetadataExt, Empty, ExecuteExt, QueryExt>;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: Cw721BaseInstantiateMsg,
) -> Result<Response, ContractError> {
    Cw721Roles::default().instantiate(deps.branch(), env.clone(), info, msg)?;

    // Initialize total weight to zero
    TOTAL.save(deps.storage, &0, env.block.height)?;

    cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::default()
        .add_attribute("contract_name", CONTRACT_NAME)
        .add_attribute("contract_version", CONTRACT_VERSION))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    // Only owner / minter can execute
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
            ExecuteExt::UpdateTokenRole { token_id, role } => {
                execute_update_token_role(deps, env, info, token_id, role)
            }
            ExecuteExt::UpdateTokenUri {
                token_id,
                token_uri,
            } => execute_update_token_uri(deps, env, info, token_id, token_uri),
            ExecuteExt::UpdateTokenWeight { token_id, weight } => {
                execute_update_token_weight(deps, env, info, token_id, weight)
            }
        },
        ExecuteMsg::TransferNft {
            recipient,
            token_id,
        } => execute_transfer(deps, env, info, recipient, token_id),
        ExecuteMsg::SendNft {
            contract,
            token_id,
            msg,
        } => execute_send(deps, env, info, token_id, contract, msg),
        _ => Cw721Roles::default()
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

    // Call base mint
    let res = Cw721Roles::default().execute(
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
    let owner: OwnerOfResponse = from_json(Cw721Roles::default().query(
        deps.as_ref(),
        env.clone(),
        QueryMsg::OwnerOf {
            token_id: token_id.clone(),
            include_expired: None,
        },
    )?)?;

    // Get the weight of the token
    let nft_info: NftInfoResponse<MetadataExt> = from_json(Cw721Roles::default().query(
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
    let old_weight = MEMBERS.load(deps.storage, &owner_addr)?;

    // Subtract the nft weight from the member's old weight
    let new_weight = old_weight
        .checked_sub(nft_info.extension.weight)
        .ok_or(ContractError::CannotBurn {})?;

    // Subtract nft weight from the total
    total = total.checked_sub(Uint64::from(nft_info.extension.weight))?;

    // Check if the new weight is now zero
    if new_weight == 0 {
        // New weight is now None
        diff = MemberDiff::new(owner.owner, Some(old_weight), None);
        // Remove owner from list of members
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

    TOTAL.save(deps.storage, &total.u64(), env.block.height)?;

    let diffs = MemberChangedHookMsg { diffs: vec![diff] };

    // Prepare hook messages
    let msgs = HOOKS.prepare_hooks(deps.storage, |h| {
        diffs.clone().into_cosmos_msg(h).map(SubMsg::new)
    })?;

    // Remove the token
    Cw721Roles::default()
        .tokens
        .remove(deps.storage, &token_id)?;
    // Decrement the account
    Cw721Roles::default().decrement_tokens(deps.storage)?;

    Ok(Response::new()
        .add_attribute("action", "burn")
        .add_attribute("sender", info.sender)
        .add_attribute("token_id", token_id)
        .add_submessages(msgs))
}

pub fn execute_transfer(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    recipient: String,
    token_id: String,
) -> Result<Response, ContractError> {
    let contract = Cw721Roles::default();

    let mut token = contract.tokens.load(deps.storage, &token_id)?;
    // set owner and remove existing approvals
    token.owner = deps.api.addr_validate(&recipient)?;
    token.approvals = vec![];
    contract.tokens.save(deps.storage, &token_id, &token)?;

    Ok(Response::new()
        .add_attribute("action", "transfer_nft")
        .add_attribute("sender", info.sender)
        .add_attribute("recipient", recipient)
        .add_attribute("token_id", token_id))
}

pub fn execute_send(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    token_id: String,
    recipient_contract: String,
    msg: Binary,
) -> Result<Response, ContractError> {
    let contract = Cw721Roles::default();

    let mut token = contract.tokens.load(deps.storage, &token_id)?;
    // set owner and remove existing approvals
    token.owner = deps.api.addr_validate(&recipient_contract)?;
    token.approvals = vec![];
    contract.tokens.save(deps.storage, &token_id, &token)?;

    let send = Cw721ReceiveMsg {
        sender: info.sender.to_string(),
        token_id: token_id.clone(),
        msg,
    };

    Ok(Response::new()
        .add_message(send.into_cosmos_msg(recipient_contract.clone())?)
        .add_attribute("action", "send_nft")
        .add_attribute("sender", info.sender)
        .add_attribute("recipient", recipient_contract)
        .add_attribute("token_id", token_id))
}

pub fn execute_add_hook(
    deps: DepsMut,
    _info: MessageInfo,
    addr: String,
) -> Result<Response, ContractError> {
    let hook = deps.api.addr_validate(&addr)?;
    HOOKS.add_hook(deps.storage, hook)?;

    Ok(Response::default()
        .add_attribute("action", "add_hook")
        .add_attribute("hook", addr))
}

pub fn execute_remove_hook(
    deps: DepsMut,
    _info: MessageInfo,
    addr: String,
) -> Result<Response, ContractError> {
    let hook = deps.api.addr_validate(&addr)?;
    HOOKS.remove_hook(deps.storage, hook)?;

    Ok(Response::default()
        .add_attribute("action", "remove_hook")
        .add_attribute("hook", addr))
}

pub fn execute_update_token_role(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    token_id: String,
    role: Option<String>,
) -> Result<Response, ContractError> {
    let contract = Cw721Roles::default();

    // Make sure NFT exists
    let mut token = contract.tokens.load(deps.storage, &token_id)?;

    // Update role with new value
    token.extension.role = role.clone();
    contract.tokens.save(deps.storage, &token_id, &token)?;

    Ok(Response::default()
        .add_attribute("action", "update_token_role")
        .add_attribute("sender", info.sender)
        .add_attribute("token_id", token_id)
        .add_attribute("role", role.unwrap_or_default()))
}

pub fn execute_update_token_uri(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    token_id: String,
    token_uri: Option<String>,
) -> Result<Response, ContractError> {
    let contract = Cw721Roles::default();

    let mut token = contract.tokens.load(deps.storage, &token_id)?;

    // Set new token URI
    token.token_uri = token_uri.clone();
    contract.tokens.save(deps.storage, &token_id, &token)?;

    Ok(Response::new()
        .add_attribute("action", "update_token_uri")
        .add_attribute("sender", info.sender)
        .add_attribute("token_id", token_id)
        .add_attribute("token_uri", token_uri.unwrap_or_default()))
}

pub fn execute_update_token_weight(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    token_id: String,
    weight: u64,
) -> Result<Response, ContractError> {
    let contract = Cw721Roles::default();

    // Make sure NFT exists
    let mut token = contract.tokens.load(deps.storage, &token_id)?;

    let mut total = Uint64::from(TOTAL.load(deps.storage)?);
    let mut diff = MemberDiff::new(token.clone().owner, None, None);

    // Update member weights and total
    MEMBERS.update(
        deps.storage,
        &token.owner,
        env.block.height,
        |old| -> Result<_, ContractError> {
            let new_total_weight;
            let old_total_weight = old.unwrap_or_default();

            // Check if new token weight is great than, less than, or equal to
            // the old token weight
            match weight.cmp(&token.extension.weight) {
                Ordering::Greater => {
                    // Subtract the old token weight from the new token weight
                    let weight_difference = weight
                        .checked_sub(token.extension.weight)
                        .ok_or(ContractError::NegativeValue {})?;

                    // Increment the total weight by the weight difference of the new token
                    total = total.checked_add(Uint64::from(weight_difference))?;
                    // Add the new NFT weight to the old weight for the owner
                    new_total_weight = old_total_weight + weight_difference;
                    // Set the diff for use in hooks
                    diff = MemberDiff::new(token.clone().owner, old, Some(new_total_weight));
                }
                Ordering::Less => {
                    // Subtract the new token weight from the old token weight
                    let weight_difference = token
                        .extension
                        .weight
                        .checked_sub(weight)
                        .ok_or(ContractError::NegativeValue {})?;

                    // Subtract the weight difference from the old total weight
                    new_total_weight = old_total_weight
                        .checked_sub(weight_difference)
                        .ok_or(ContractError::NegativeValue {})?;

                    // Subtract difference from the total
                    total = total.checked_sub(Uint64::from(weight_difference))?;
                }
                Ordering::Equal => return Err(ContractError::NoWeightChange {}),
            }

            Ok(new_total_weight)
        },
    )?;
    TOTAL.save(deps.storage, &total.u64(), env.block.height)?;

    let diffs = MemberChangedHookMsg { diffs: vec![diff] };

    // Prepare hook messages
    let msgs = HOOKS.prepare_hooks(deps.storage, |h| {
        diffs.clone().into_cosmos_msg(h).map(SubMsg::new)
    })?;

    // Save token weight
    token.extension.weight = weight;
    contract.tokens.save(deps.storage, &token_id, &token)?;

    Ok(Response::default()
        .add_submessages(msgs)
        .add_attribute("action", "update_token_weight")
        .add_attribute("sender", info.sender)
        .add_attribute("token_id", token_id)
        .add_attribute("weight", weight.to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Extension { msg } => match msg {
            QueryExt::Hooks {} => to_json_binary(&HOOKS.query_hooks(deps)?),
            QueryExt::ListMembers { start_after, limit } => {
                to_json_binary(&query_list_members(deps, start_after, limit)?)
            }
            QueryExt::Member { addr, at_height } => {
                to_json_binary(&query_member(deps, addr, at_height)?)
            }
            QueryExt::TotalWeight { at_height } => {
                to_json_binary(&query_total_weight(deps, at_height)?)
            }
        },
        _ => Cw721Roles::default().query(deps, env, msg),
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
