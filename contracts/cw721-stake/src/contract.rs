use crate::hooks::{stake_hook_msgs, unstake_hook_msgs};
#[cfg(not(feature = "library"))]
use crate::msg::{
    ExecuteMsg, GetHooksResponse, InstantiateMsg, Owner, QueryMsg, StakedBalanceAtHeightResponse,
    TotalStakedAtHeightResponse,
};
use crate::state::{
    Config, CONFIG, HOOKS, MAX_CLAIMS, NFT_CLAIMS, STAKED_NFTS_PER_OWNER, TOTAL_STAKED_NFTS,
};
use crate::ContractError;
use cosmwasm_std::{
    entry_point, to_binary, Addr, Binary, CosmosMsg, Deps, DepsMut, Empty, Env, MessageInfo,
    Response, StdError, StdResult, Uint128, WasmMsg,
};
use cw2::set_contract_version;
use cw721::Cw721ReceiveMsg;
use cw_utils::Duration;
use indexmap::IndexSet;
use std::convert::{From, TryFrom};

pub(crate) const CONTRACT_NAME: &str = "crates.io:cw721_stake";
pub(crate) const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response<Empty>, ContractError> {
    let owner = msg
        .owner
        .as_ref()
        .map(|owner| match owner {
            Owner::Addr(address) => deps.api.addr_validate(address),
            Owner::Instantiator {} => Ok(info.sender),
        })
        .transpose()?;
    let manager = msg
        .manager
        .as_ref()
        .map(|h| deps.api.addr_validate(h))
        .transpose()?;

    let config = Config {
        owner: owner.clone(),
        manager,
        nft_address: deps.api.addr_validate(&msg.nft_address)?,
        unstaking_duration: msg.unstaking_duration,
    };
    CONFIG.save(deps.storage, &config)?;
    TOTAL_STAKED_NFTS.save(deps.storage, &Uint128::zero(), env.block.height)?;
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::default()
        .add_attribute("method", "instantiate")
        .add_attribute("nft_contract", msg.nft_address)
        .add_attribute(
            "owner",
            owner
                .map(|a| a.into_string())
                .unwrap_or_else(|| "None".to_string()),
        )
        .add_attribute("manager", msg.manager.unwrap_or_else(|| "None".to_string())))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response<Empty>, ContractError> {
    match msg {
        ExecuteMsg::ReceiveNft(msg) => execute_stake(deps, env, info, msg),
        ExecuteMsg::Unstake { token_ids } => execute_unstake(deps, env, info, token_ids),
        ExecuteMsg::ClaimNfts {} => execute_claim_nfts(deps, env, info),
        ExecuteMsg::UpdateConfig {
            owner,
            manager,
            duration,
        } => execute_update_config(info, deps, owner, manager, duration),
        ExecuteMsg::AddHook { addr } => execute_add_hook(deps, env, info, addr),
        ExecuteMsg::RemoveHook { addr } => execute_remove_hook(deps, env, info, addr),
    }
}

pub fn execute_stake(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    wrapper: Cw721ReceiveMsg,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.nft_address {
        return Err(ContractError::InvalidToken {
            received: info.sender,
            expected: config.nft_address,
        });
    }

    let sender = deps.api.addr_validate(&wrapper.sender)?;

    STAKED_NFTS_PER_OWNER.update(
        deps.storage,
        sender.clone(),
        env.block.height,
        |nft_collection| -> StdResult<IndexSet<String>> {
            let mut updated_nft_collection = nft_collection.unwrap_or_default();
            updated_nft_collection.insert(wrapper.token_id.clone());
            Ok(updated_nft_collection)
        },
    )?;

    TOTAL_STAKED_NFTS.update(
        deps.storage,
        env.block.height,
        |total_staked| -> StdResult<_> {
            total_staked
                .unwrap()
                .checked_add(Uint128::new(1))
                .map_err(StdError::overflow)
        },
    )?;

    let hook_msgs = stake_hook_msgs(deps.storage, sender.clone(), wrapper.token_id.clone())?;
    Ok(Response::default()
        .add_submessages(hook_msgs)
        .add_attribute("action", "stake")
        .add_attribute("from", sender)
        .add_attribute("token_id", wrapper.token_id))
}

pub fn execute_unstake(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    token_ids: Vec<String>,
) -> Result<Response, ContractError> {
    if token_ids.is_empty() {
        return Err(ContractError::ZeroUnstake {});
    }

    let config = CONFIG.load(deps.storage)?;

    let resulting_collection = STAKED_NFTS_PER_OWNER.update(
        deps.storage,
        info.sender.clone(),
        env.block.height,
        |nft_collection| -> Result<IndexSet<String>, ContractError> {
            if let Some(mut nft_collection) = nft_collection {
                // Some benchmarking suggests this is actually the
                // fastest way to remove a list of items from a
                // HashSet.
                //
                // Alternatives include, `drain().filter()` and
                // `difference(&to_remove.into_iter().collect()).cloned().collect()`. Difference
                // here, suprisingly, being ~2x the speed of drain and
                // filter. Remove in a loop clocks in at ~2x the speed
                // of difference.
                for token_id in token_ids.iter() {
                    // This will implicitly check for duplicates in
                    // the input vector as removing twice will fail
                    // the second time around.
                    let was_present = nft_collection.remove(token_id);
                    if !was_present {
                        // Can't unstake that which you do not own.
                        return Err(ContractError::NotStaked {});
                    }
                }
                Ok(nft_collection)
            } else {
                // Has never staked anything.
                Err(ContractError::NotStaked {})
            }
        },
    )?;

    // If this would result in the staker having no more staked NFTs,
    // remove them from the storage map. This stops queries that
    // enumerate the staker list from getting a bunch of stakers who
    // have zero staked.
    if resulting_collection.is_empty() {
        STAKED_NFTS_PER_OWNER.remove(deps.storage, info.sender.clone(), env.block.height)?;
    }

    TOTAL_STAKED_NFTS.update(
        deps.storage,
        env.block.height,
        |total_staked| -> StdResult<_> {
            total_staked
                .unwrap()
                .checked_sub(Uint128::new(token_ids.len() as u128))
                .map_err(StdError::overflow)
        },
    )?;

    let hook_msgs = unstake_hook_msgs(deps.storage, info.sender.clone(), token_ids.clone())?;
    match config.unstaking_duration {
        None => {
            let return_messages = token_ids
                .into_iter()
                .map(|token_id| -> StdResult<WasmMsg> {
                    Ok(cosmwasm_std::WasmMsg::Execute {
                        contract_addr: config.nft_address.to_string(),
                        msg: to_binary(&cw721::Cw721ExecuteMsg::TransferNft {
                            recipient: info.sender.to_string(),
                            token_id,
                        })?,
                        funds: vec![],
                    })
                })
                .collect::<StdResult<Vec<_>>>()?;

            Ok(Response::default()
                .add_messages(return_messages)
                .add_submessages(hook_msgs)
                .add_attribute("action", "unstake")
                .add_attribute("from", info.sender)
                .add_attribute("claim_duration", "None"))
        }

        Some(duration) => {
            let outstanding_claims = NFT_CLAIMS
                .query_claims(deps.as_ref(), &info.sender)?
                .nft_claims;
            if outstanding_claims.len() >= MAX_CLAIMS as usize {
                return Err(ContractError::TooManyClaims {});
            }

            // Out of gas here is fine - just try again with fewer
            // tokens.
            NFT_CLAIMS.create_nft_claims(
                deps.storage,
                &info.sender,
                token_ids,
                duration.after(&env.block),
            )?;

            Ok(Response::default()
                .add_attribute("action", "unstake")
                .add_submessages(hook_msgs)
                .add_attribute("from", info.sender)
                .add_attribute("claim_duration", format!("{}", duration)))
        }
    }
}

pub fn execute_claim_nfts(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let nfts = NFT_CLAIMS.claim_nfts(deps.storage, &info.sender, &_env.block)?;
    if nfts.is_empty() {
        return Err(ContractError::NothingToClaim {});
    }

    let config = CONFIG.load(deps.storage)?;

    let msgs = nfts
        .into_iter()
        .map(|nft| -> StdResult<CosmosMsg> {
            Ok(WasmMsg::Execute {
                contract_addr: config.nft_address.to_string(),
                msg: to_binary(&cw721::Cw721ExecuteMsg::TransferNft {
                    recipient: info.sender.to_string(),
                    token_id: nft,
                })?,
                funds: vec![],
            }
            .into())
        })
        .collect::<StdResult<Vec<_>>>()?;

    Ok(Response::default()
        .add_messages(msgs)
        .add_attribute("action", "claim_nfts")
        .add_attribute("from", info.sender))
}

pub fn execute_update_config(
    info: MessageInfo,
    deps: DepsMut,
    new_owner: Option<String>,
    new_manager: Option<String>,
    duration: Option<Duration>,
) -> Result<Response, ContractError> {
    let new_owner = new_owner
        .map(|new_owner| deps.api.addr_validate(&new_owner))
        .transpose()?;
    let new_manager = new_manager
        .map(|new_manager| deps.api.addr_validate(&new_manager))
        .transpose()?;

    let mut config: Config = CONFIG.load(deps.storage)?;
    if Some(info.sender.clone()) != config.owner && Some(info.sender.clone()) != config.manager {
        return Err(ContractError::Unauthorized {});
    };

    if Some(info.sender) != config.owner && new_owner != config.owner {
        return Err(ContractError::OnlyOwnerCanChangeOwner {});
    };

    config.owner = new_owner;
    config.manager = new_manager;
    config.unstaking_duration = duration;
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::default()
        .add_attribute("action", "update_config")
        .add_attribute(
            "owner",
            config
                .owner
                .map(|a| a.to_string())
                .unwrap_or_else(|| "None".to_string()),
        )
        .add_attribute(
            "manager",
            config
                .manager
                .map(|a| a.to_string())
                .unwrap_or_else(|| "None".to_string()),
        ))
}

pub fn execute_add_hook(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    addr: String,
) -> Result<Response, ContractError> {
    let addr = deps.api.addr_validate(&addr)?;
    let config: Config = CONFIG.load(deps.storage)?;
    if config.owner != Some(info.sender.clone()) && config.manager != Some(info.sender) {
        return Err(ContractError::Unauthorized {});
    };

    HOOKS.add_hook(deps.storage, addr.clone())?;

    Ok(Response::default()
        .add_attribute("action", "add_hook")
        .add_attribute("hook", addr))
}

pub fn execute_remove_hook(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    addr: String,
) -> Result<Response, ContractError> {
    let addr = deps.api.addr_validate(&addr)?;
    let config: Config = CONFIG.load(deps.storage)?;
    if config.owner != Some(info.sender.clone()) && config.manager != Some(info.sender) {
        return Err(ContractError::Unauthorized {});
    };

    HOOKS.remove_hook(deps.storage, addr.clone())?;

    Ok(Response::default()
        .add_attribute("action", "remove_hook")
        .add_attribute("hook", addr))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetConfig {} => query_config(deps),
        QueryMsg::StakedBalanceAtHeight { address, height } => {
            query_staked_balance_at_height(deps, env, address, height)
        }
        QueryMsg::TotalStakedAtHeight { height } => query_total_staked_at_height(deps, env, height),
        QueryMsg::NftClaims { address } => query_nft_claims(deps, address),
        QueryMsg::GetHooks {} => query_hooks(deps),
        QueryMsg::VotingPowerAtHeight { address, height } => {
            query_voting_power_at_height(deps, env, address, height)
        }
        QueryMsg::TotalPowerAtHeight { height } => query_total_power_at_height(deps, env, height),
        QueryMsg::Info {} => query_info(deps),
        QueryMsg::ListStakers { start_after, limit } => {
            query_list_stakers(deps, start_after, limit)
        }
        QueryMsg::StakedNfts {
            address,
            start_after,
            limit,
        } => query_staked_nfts(deps, address, start_after, limit),
    }
}

pub fn query_staked_balance_at_height(
    deps: Deps,
    env: Env,
    address: String,
    height: Option<u64>,
) -> StdResult<Binary> {
    let address = deps.api.addr_validate(&address)?;
    let height = height.unwrap_or(env.block.height);
    let nft_collection = STAKED_NFTS_PER_OWNER
        .may_load_at_height(deps.storage, address, height)?
        .unwrap_or_default();

    to_binary(&StakedBalanceAtHeightResponse {
        balance: Uint128::from(u128::try_from(nft_collection.len()).unwrap()),
        height,
    })
}

pub fn query_voting_power_at_height(
    deps: Deps,
    env: Env,
    address: String,
    height: Option<u64>,
) -> StdResult<Binary> {
    let address = deps.api.addr_validate(&address)?;
    let height = height.unwrap_or(env.block.height);
    let collection = STAKED_NFTS_PER_OWNER
        .may_load_at_height(deps.storage, address, height)?
        .unwrap_or_default();
    let power = Uint128::new(collection.len() as u128);

    to_binary(&cw_core_interface::voting::VotingPowerAtHeightResponse { power, height })
}

pub fn query_total_staked_at_height(
    deps: Deps,
    env: Env,
    height: Option<u64>,
) -> StdResult<Binary> {
    let height = height.unwrap_or(env.block.height);
    let total_staked_nfts = TOTAL_STAKED_NFTS
        .may_load_at_height(deps.storage, height)?
        .unwrap_or_default();

    to_binary(&TotalStakedAtHeightResponse {
        total: total_staked_nfts,
        height,
    })
}

pub fn query_total_power_at_height(deps: Deps, env: Env, height: Option<u64>) -> StdResult<Binary> {
    let height = height.unwrap_or(env.block.height);
    let power = TOTAL_STAKED_NFTS
        .may_load_at_height(deps.storage, height)?
        .unwrap_or_default();
    to_binary(&cw_core_interface::voting::TotalPowerAtHeightResponse { power, height })
}

pub fn query_config(deps: Deps) -> StdResult<Binary> {
    let config = CONFIG.load(deps.storage)?;
    to_binary(&config)
}

pub fn query_nft_claims(deps: Deps, address: String) -> StdResult<Binary> {
    to_binary(&NFT_CLAIMS.query_claims(deps, &deps.api.addr_validate(&address)?)?)
}

pub fn query_hooks(deps: Deps) -> StdResult<Binary> {
    to_binary(&GetHooksResponse {
        hooks: HOOKS.query_hooks(deps)?.hooks,
    })
}

pub fn query_info(deps: Deps) -> StdResult<Binary> {
    let info = cw2::get_contract_version(deps.storage)?;
    to_binary(&cw_core_interface::voting::InfoResponse { info })
}

pub fn query_list_stakers(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Binary> {
    let start_at = start_after
        .map(|addr| deps.api.addr_validate(&addr))
        .transpose()?;

    // Type decoration here isn't strictly needed but we want to make
    // sure the return type of this query doesn't change due to a code
    // change elsewhere that gets hidden away by generics.
    let res: Vec<Addr> = cw_paginate::paginate_snapshot_map_keys(
        deps,
        &STAKED_NFTS_PER_OWNER,
        start_at,
        limit,
        cosmwasm_std::Order::Descending,
    )?;

    to_binary(&res)
}

pub fn query_staked_nfts(
    deps: Deps,
    address: String,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Binary> {
    let address = deps.api.addr_validate(&address)?;
    let nfts = STAKED_NFTS_PER_OWNER.load(deps.storage, address)?;

    let start_index: usize = start_after
        .map(|start_after| {
            nfts.get_index_of(&start_after)
                // Want index + 1 with max possible value being the
                // highest index in the map.
                .map(|index| (index + 1).min(nfts.len().saturating_sub(1)))
                .ok_or(StdError::NotFound { kind: start_after })
        })
        .transpose()?
        .unwrap_or_default();

    let end_index: usize = limit
        .map(|limit| (start_index + limit as usize).min(nfts.len()))
        .unwrap_or_else(|| nfts.len());

    // Allocate only as much space as we need.
    let mut res: Vec<String> = Vec::with_capacity(end_index - start_index);

    for index in start_index..end_index {
        // Safe to unwrap here as the above code has already checked
        // that these indexes are in bounds. This is constant time.
        res.push(nfts.get_index(index).unwrap().clone())
    }

    to_binary(&res)
}
