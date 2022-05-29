#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, CosmosMsg, Deps, DepsMut, Empty, Env, MessageInfo, Reply, Response,
    StdResult, SubMsg,
};
use cw2::{get_contract_version, set_contract_version};
use cw_storage_plus::Map;
use cw_utils::{parse_reply_instantiate_data, Duration};

use cw_core_interface::voting;
use cw_paginate::{paginate_map, paginate_map_keys};

use crate::error::ContractError;
use crate::msg::{
    ExecuteMsg, InitialItem, InstantiateMsg, MigrateMsg, ModuleInstantiateInfo, QueryMsg,
};
use crate::query::{Cw20BalanceResponse, DumpStateResponse, GetItemResponse, PauseInfoResponse};
use crate::state::{
    Config, ADMIN, CONFIG, CW20_LIST, CW721_LIST, ITEMS, PAUSED, PROPOSAL_MODULES, VOTING_MODULE,
};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cw-core";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const PROPOSAL_MODULE_REPLY_ID: u64 = 0;
const VOTE_MODULE_INSTANTIATE_REPLY_ID: u64 = 1;
const VOTE_MODULE_UPDATE_REPLY_ID: u64 = 2;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let config = Config {
        name: msg.name,
        description: msg.description,
        image_url: msg.image_url,
        automatically_add_cw20s: msg.automatically_add_cw20s,
        automatically_add_cw721s: msg.automatically_add_cw721s,
    };
    CONFIG.save(deps.storage, &config)?;

    let admin = msg
        .admin
        .map(|human| deps.api.addr_validate(&human))
        .transpose()?;
    ADMIN.save(deps.storage, &admin)?;

    let vote_module_msg = msg
        .voting_module_instantiate_info
        .into_wasm_msg(env.contract.address.clone());
    let vote_module_msg: SubMsg<Empty> =
        SubMsg::reply_on_success(vote_module_msg, VOTE_MODULE_INSTANTIATE_REPLY_ID);

    let proposal_module_msgs: Vec<SubMsg<Empty>> = msg
        .proposal_modules_instantiate_info
        .into_iter()
        .map(|info| info.into_wasm_msg(env.contract.address.clone()))
        .map(|wasm| SubMsg::reply_on_success(wasm, PROPOSAL_MODULE_REPLY_ID))
        .collect();
    if proposal_module_msgs.is_empty() {
        return Err(ContractError::NoProposalModule {});
    }

    for InitialItem { key, value } in msg.initial_items.unwrap_or_default() {
        ITEMS.save(deps.storage, key, &value)?;
    }

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("sender", info.sender)
        .add_submessage(vote_module_msg)
        .add_submessages(proposal_module_msgs))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    // No actions can be performed while the DAO is paused.
    if let Some(expiration) = PAUSED.may_load(deps.storage)? {
        if !expiration.is_expired(&env.block) {
            return Err(ContractError::Paused {});
        }
    }

    match msg {
        ExecuteMsg::ExecuteAdminMsgs { msgs } => {
            execute_admin_msgs(deps.as_ref(), info.sender, msgs)
        }
        ExecuteMsg::ExecuteProposalHook { msgs } => {
            execute_proposal_hook(deps.as_ref(), info.sender, msgs)
        }
        ExecuteMsg::Pause { duration } => execute_pause(deps, env, info.sender, duration),
        ExecuteMsg::Receive(_) => execute_receive_cw20(deps, info.sender),
        ExecuteMsg::ReceiveNft(_) => execute_receive_cw721(deps, info.sender),
        ExecuteMsg::RemoveItem { key } => execute_remove_item(deps, env, info.sender, key),
        ExecuteMsg::SetItem { key, addr } => execute_set_item(deps, env, info.sender, key, addr),
        ExecuteMsg::UpdateAdmin { admin } => execute_update_admin(deps, info.sender, admin),
        ExecuteMsg::UpdateConfig { config } => {
            execute_update_config(deps, env, info.sender, config)
        }
        ExecuteMsg::UpdateCw20List { to_add, to_remove } => {
            execute_update_cw20_list(deps, env, info.sender, to_add, to_remove)
        }
        ExecuteMsg::UpdateCw721List { to_add, to_remove } => {
            execute_update_cw721_list(deps, env, info.sender, to_add, to_remove)
        }
        ExecuteMsg::UpdateVotingModule { module } => {
            execute_update_voting_module(env, info.sender, module)
        }
        ExecuteMsg::UpdateProposalModules { to_add, to_remove } => {
            execute_update_proposal_modules(deps, env, info.sender, to_add, to_remove)
        }
    }
}

pub fn execute_pause(
    deps: DepsMut,
    env: Env,
    sender: Addr,
    pause_duration: Duration,
) -> Result<Response, ContractError> {
    // Only the core contract may call this method.
    if sender != env.contract.address {
        return Err(ContractError::Unauthorized {});
    }

    let until = pause_duration.after(&env.block);

    PAUSED.save(deps.storage, &until)?;

    Ok(Response::new()
        .add_attribute("action", "execute_pause")
        .add_attribute("sender", sender)
        .add_attribute("until", until.to_string()))
}

pub fn execute_admin_msgs(
    deps: Deps,
    sender: Addr,
    msgs: Vec<CosmosMsg<Empty>>,
) -> Result<Response, ContractError> {
    let admin = ADMIN.load(deps.storage)?;

    match admin {
        Some(admin) => {
            // Check if the sender is the DAO Admin
            if sender != admin {
                return Err(ContractError::Unauthorized {});
            }

            Ok(Response::default()
                .add_attribute("action", "execute_admin_msgs")
                .add_messages(msgs))
        }
        None => Err(ContractError::NoAdmin {}),
    }
}

pub fn execute_proposal_hook(
    deps: Deps,
    sender: Addr,
    msgs: Vec<CosmosMsg<Empty>>,
) -> Result<Response, ContractError> {
    // Check that the message has come from one of the proposal modules
    if !PROPOSAL_MODULES.has(deps.storage, sender) {
        return Err(ContractError::Unauthorized {});
    }

    Ok(Response::default()
        .add_attribute("action", "execute_proposal_hook")
        .add_messages(msgs))
}

pub fn execute_update_admin(
    deps: DepsMut,
    sender: Addr,
    admin: Option<Addr>,
) -> Result<Response, ContractError> {
    let current_admin = ADMIN.load(deps.storage)?;

    match current_admin {
        Some(current_admin) => {
            // Check sender is the DAO Admin
            if sender != current_admin {
                return Err(ContractError::Unauthorized {});
            }

            // Save the new DAO Admin (which may be set to None)
            ADMIN.save(deps.storage, &admin)?;

            Ok(Response::default()
                .add_attribute("action", "execute_update_admin")
                .add_attribute(
                    "new_admin",
                    admin
                        .map(|a| a.into_string())
                        .unwrap_or_else(|| "None".to_string()),
                ))
        }
        None => {
            // If no DAO admin is configured, return unauthorized
            Err(ContractError::Unauthorized {})
        }
    }
}

pub fn execute_update_config(
    deps: DepsMut,
    env: Env,
    sender: Addr,
    config: Config,
) -> Result<Response, ContractError> {
    if sender != env.contract.address {
        return Err(ContractError::Unauthorized {});
    }

    CONFIG.save(deps.storage, &config)?;
    // We incur some gas costs by having the config's fields in the
    // response. This has the benefit that it makes it reasonably
    // simple to ask "when did this field in the config change" by
    // running something like `junod query txs --events
    // 'wasm._contract_address=core&wasm.name=name'`.
    Ok(Response::default()
        .add_attribute("action", "execute_update_config")
        .add_attribute("name", config.name)
        .add_attribute("description", config.description)
        .add_attribute(
            "image_url",
            config.image_url.unwrap_or_else(|| "None".to_string()),
        ))
}

pub fn execute_update_voting_module(
    env: Env,
    sender: Addr,
    module: ModuleInstantiateInfo,
) -> Result<Response, ContractError> {
    if env.contract.address != sender {
        return Err(ContractError::Unauthorized {});
    }

    let wasm = module.into_wasm_msg(env.contract.address);
    let submessage = SubMsg::reply_on_success(wasm, VOTE_MODULE_UPDATE_REPLY_ID);

    Ok(Response::default()
        .add_attribute("action", "execute_update_voting_module")
        .add_submessage(submessage))
}

pub fn execute_update_proposal_modules(
    deps: DepsMut,
    env: Env,
    sender: Addr,
    to_add: Vec<ModuleInstantiateInfo>,
    to_remove: Vec<String>,
) -> Result<Response, ContractError> {
    if env.contract.address != sender {
        return Err(ContractError::Unauthorized {});
    }

    for addr in to_remove {
        let addr = deps.api.addr_validate(&addr)?;
        PROPOSAL_MODULES.remove(deps.storage, addr);
    }

    let to_add: Vec<SubMsg<Empty>> = to_add
        .into_iter()
        .map(|info| info.into_wasm_msg(env.contract.address.clone()))
        .map(|wasm| SubMsg::reply_on_success(wasm, PROPOSAL_MODULE_REPLY_ID))
        .collect();

    // If we removed all of our proposal modules and we are not adding
    // any this operation would result in no proposal modules being
    // present.
    if PROPOSAL_MODULES
        .keys_raw(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .next()
        .is_none()
        && to_add.is_empty()
    {
        return Err(ContractError::NoProposalModule {});
    }

    Ok(Response::default()
        .add_attribute("action", "execute_update_proposal_modules")
        .add_submessages(to_add))
}

fn do_update_addr_list(
    deps: DepsMut,
    map: Map<Addr, Empty>,
    to_add: Vec<String>,
    to_remove: Vec<String>,
) -> Result<(), ContractError> {
    let to_add = to_add
        .into_iter()
        .map(|a| deps.api.addr_validate(&a))
        .collect::<Result<Vec<_>, _>>()?;

    let to_remove = to_remove
        .into_iter()
        .map(|a| deps.api.addr_validate(&a))
        .collect::<Result<Vec<_>, _>>()?;

    for addr in to_add {
        map.save(deps.storage, addr, &Empty {})?;
    }
    for addr in to_remove {
        map.remove(deps.storage, addr);
    }

    Ok(())
}

pub fn execute_update_cw20_list(
    deps: DepsMut,
    env: Env,
    sender: Addr,
    to_add: Vec<String>,
    to_remove: Vec<String>,
) -> Result<Response, ContractError> {
    if env.contract.address != sender {
        return Err(ContractError::Unauthorized {});
    }
    do_update_addr_list(deps, CW20_LIST, to_add, to_remove)?;
    Ok(Response::default().add_attribute("action", "update_cw20_list"))
}

pub fn execute_update_cw721_list(
    deps: DepsMut,
    env: Env,
    sender: Addr,
    to_add: Vec<String>,
    to_remove: Vec<String>,
) -> Result<Response, ContractError> {
    if env.contract.address != sender {
        return Err(ContractError::Unauthorized {});
    }
    do_update_addr_list(deps, CW721_LIST, to_add, to_remove)?;
    Ok(Response::default().add_attribute("action", "update_cw721_list"))
}

pub fn execute_set_item(
    deps: DepsMut,
    env: Env,
    sender: Addr,
    key: String,
    value: String,
) -> Result<Response, ContractError> {
    if env.contract.address != sender {
        return Err(ContractError::Unauthorized {});
    }

    ITEMS.save(deps.storage, key.clone(), &value)?;
    Ok(Response::default()
        .add_attribute("action", "execute_set_item")
        .add_attribute("key", key)
        .add_attribute("addr", value))
}

pub fn execute_remove_item(
    deps: DepsMut,
    env: Env,
    sender: Addr,
    key: String,
) -> Result<Response, ContractError> {
    if env.contract.address != sender {
        return Err(ContractError::Unauthorized {});
    }

    ITEMS.remove(deps.storage, key.clone());
    Ok(Response::default()
        .add_attribute("action", "execute_remove_item")
        .add_attribute("key", key))
}

pub fn execute_receive_cw20(deps: DepsMut, sender: Addr) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if !config.automatically_add_cw20s {
        Ok(Response::new())
    } else {
        CW20_LIST.save(deps.storage, sender.clone(), &Empty {})?;
        Ok(Response::new()
            .add_attribute("action", "receive_cw20")
            .add_attribute("token", sender))
    }
}

pub fn execute_receive_cw721(deps: DepsMut, sender: Addr) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if !config.automatically_add_cw721s {
        Ok(Response::new())
    } else {
        CW721_LIST.save(deps.storage, sender.clone(), &Empty {})?;
        Ok(Response::new()
            .add_attribute("action", "receive_cw721")
            .add_attribute("token", sender))
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Admin {} => query_admin(deps),
        QueryMsg::Config {} => query_config(deps),
        QueryMsg::Cw20TokenList { start_at, limit } => query_cw20_list(deps, start_at, limit),
        QueryMsg::Cw20Balances { start_at, limit } => {
            query_cw20_balances(deps, env, start_at, limit)
        }
        QueryMsg::Cw721TokenList { start_at, limit } => query_cw721_list(deps, start_at, limit),
        QueryMsg::DumpState {} => query_dump_state(deps, env),
        QueryMsg::GetItem { key } => query_get_item(deps, key),
        QueryMsg::Info {} => query_info(deps),
        QueryMsg::ListItems { start_at, limit } => query_list_items(deps, start_at, limit),
        QueryMsg::PauseInfo {} => query_paused(deps, env),
        QueryMsg::ProposalModules { start_at, limit } => {
            query_proposal_modules(deps, start_at, limit)
        }
        QueryMsg::TotalPowerAtHeight { height } => query_total_power_at_height(deps, height),
        QueryMsg::VotingModule {} => query_voting_module(deps),
        QueryMsg::VotingPowerAtHeight { address, height } => {
            query_voting_power_at_height(deps, address, height)
        }
    }
}

pub fn query_admin(deps: Deps) -> StdResult<Binary> {
    let admin = ADMIN.load(deps.storage)?;
    to_binary(&admin)
}

pub fn query_config(deps: Deps) -> StdResult<Binary> {
    let config = CONFIG.load(deps.storage)?;
    to_binary(&config)
}

pub fn query_voting_module(deps: Deps) -> StdResult<Binary> {
    let voting_module = VOTING_MODULE.load(deps.storage)?;
    to_binary(&voting_module)
}

pub fn query_proposal_modules(
    deps: Deps,
    start_at: Option<String>,
    limit: Option<u32>,
) -> StdResult<Binary> {
    // This query is will run out of gas due to the size of the
    // returned message before it runs out of compute so taking a
    // limit here is still nice. As removes happen in constant time
    // the contract is still recoverable if too many items end up in
    // here.
    //
    // Further, as the `keys` method on a map returns an iterator it
    // is possible (though implementation dependent) that new keys are
    // loaded on demand as the iterator is moved. Should this be the
    // case we are only paying for what we need here.
    //
    // Even if this does lock up one can determine the existing
    // proposal modules by looking at past transactions on chain.
    to_binary(&paginate_map_keys(
        deps,
        &PROPOSAL_MODULES,
        start_at.map(|s| deps.api.addr_validate(&s)).transpose()?,
        limit,
        cosmwasm_std::Order::Descending,
    )?)
}

fn get_pause_info(deps: Deps, env: Env) -> StdResult<PauseInfoResponse> {
    Ok(match PAUSED.may_load(deps.storage)? {
        Some(expiration) => {
            if expiration.is_expired(&env.block) {
                PauseInfoResponse::Unpaused {}
            } else {
                PauseInfoResponse::Paused { expiration }
            }
        }
        None => PauseInfoResponse::Unpaused {},
    })
}

pub fn query_paused(deps: Deps, env: Env) -> StdResult<Binary> {
    to_binary(&get_pause_info(deps, env)?)
}

pub fn query_dump_state(deps: Deps, env: Env) -> StdResult<Binary> {
    let admin = ADMIN.load(deps.storage)?;
    let config = CONFIG.load(deps.storage)?;
    let voting_module = VOTING_MODULE.load(deps.storage)?;
    let proposal_modules = PROPOSAL_MODULES
        .keys(deps.storage, None, None, cosmwasm_std::Order::Descending)
        .collect::<Result<Vec<Addr>, _>>()?;
    let pause_info = get_pause_info(deps, env)?;
    let version = get_contract_version(deps.storage)?;
    to_binary(&DumpStateResponse {
        admin,
        config,
        version,
        pause_info,
        proposal_modules,
        voting_module,
    })
}

pub fn query_voting_power_at_height(
    deps: Deps,
    address: String,
    height: Option<u64>,
) -> StdResult<Binary> {
    let voting_module = VOTING_MODULE.load(deps.storage)?;
    let voting_power: voting::VotingPowerAtHeightResponse = deps.querier.query_wasm_smart(
        voting_module,
        &voting::Query::VotingPowerAtHeight { height, address },
    )?;
    to_binary(&voting_power)
}

pub fn query_total_power_at_height(deps: Deps, height: Option<u64>) -> StdResult<Binary> {
    let voting_module = VOTING_MODULE.load(deps.storage)?;
    let total_power: voting::TotalPowerAtHeightResponse = deps
        .querier
        .query_wasm_smart(voting_module, &voting::Query::TotalPowerAtHeight { height })?;
    to_binary(&total_power)
}

pub fn query_get_item(deps: Deps, item: String) -> StdResult<Binary> {
    let item = ITEMS.may_load(deps.storage, item)?;
    to_binary(&GetItemResponse { item })
}

pub fn query_info(deps: Deps) -> StdResult<Binary> {
    let info = cw2::get_contract_version(deps.storage)?;
    to_binary(&cw_core_interface::voting::InfoResponse { info })
}

pub fn query_list_items(
    deps: Deps,
    start_at: Option<String>,
    limit: Option<u32>,
) -> StdResult<Binary> {
    to_binary(&paginate_map(
        deps,
        &ITEMS,
        start_at,
        limit,
        cosmwasm_std::Order::Descending,
    )?)
}

pub fn query_cw20_list(
    deps: Deps,
    start_at: Option<String>,
    limit: Option<u32>,
) -> StdResult<Binary> {
    to_binary(&paginate_map_keys(
        deps,
        &CW20_LIST,
        start_at.map(|s| deps.api.addr_validate(&s)).transpose()?,
        limit,
        cosmwasm_std::Order::Descending,
    )?)
}

pub fn query_cw721_list(
    deps: Deps,
    start_at: Option<String>,
    limit: Option<u32>,
) -> StdResult<Binary> {
    to_binary(&paginate_map_keys(
        deps,
        &CW721_LIST,
        start_at.map(|s| deps.api.addr_validate(&s)).transpose()?,
        limit,
        cosmwasm_std::Order::Descending,
    )?)
}

pub fn query_cw20_balances(
    deps: Deps,
    env: Env,
    start_at: Option<String>,
    limit: Option<u32>,
) -> StdResult<Binary> {
    let addrs = paginate_map_keys(
        deps,
        &CW20_LIST,
        start_at.map(|a| deps.api.addr_validate(&a)).transpose()?,
        limit,
        cosmwasm_std::Order::Descending,
    )?;
    let balances = addrs
        .into_iter()
        .map(|addr| {
            let balance: cw20::BalanceResponse = deps.querier.query_wasm_smart(
                addr.clone(),
                &cw20::Cw20QueryMsg::Balance {
                    address: env.contract.address.to_string(),
                },
            )?;
            Ok(Cw20BalanceResponse {
                addr,
                balance: balance.balance,
            })
        })
        .collect::<StdResult<Vec<_>>>()?;
    to_binary(&balances)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    // Don't do any state migrations.
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        PROPOSAL_MODULE_REPLY_ID => {
            let res = parse_reply_instantiate_data(msg)?;
            let prop_module_addr = deps.api.addr_validate(&res.contract_address)?;
            PROPOSAL_MODULES.save(deps.storage, prop_module_addr, &Empty {})?;

            Ok(Response::default().add_attribute("prop_module".to_string(), res.contract_address))
        }
        VOTE_MODULE_INSTANTIATE_REPLY_ID => {
            let res = parse_reply_instantiate_data(msg)?;
            let vote_module_addr = deps.api.addr_validate(&res.contract_address)?;
            let current = VOTING_MODULE.may_load(deps.storage)?;

            // Make sure a bug in instantiation isn't causing us to
            // make more than one voting module.
            if current.is_some() {
                return Err(ContractError::MultipleVotingModules {});
            }

            VOTING_MODULE.save(deps.storage, &vote_module_addr)?;

            Ok(Response::default().add_attribute("voting_module", vote_module_addr))
        }
        VOTE_MODULE_UPDATE_REPLY_ID => {
            let res = parse_reply_instantiate_data(msg)?;
            let vote_module_addr = deps.api.addr_validate(&res.contract_address)?;

            VOTING_MODULE.save(deps.storage, &vote_module_addr)?;

            Ok(Response::default().add_attribute("voting_module", vote_module_addr))
        }
        _ => Err(ContractError::UnknownReplyID {}),
    }
}
