use std::convert::TryInto;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, CosmosMsg, Deps, DepsMut, Empty, Env, MessageInfo, Reply, Response,
    StdResult, SubMsg,
};
use cw2::{get_contract_version, set_contract_version};
use cw_storage_plus::{Bound, Map};
use cw_utils::parse_reply_instantiate_data;

use cw_core_interface::voting;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InitialItemInfo, InstantiateMsg, ModuleInstantiateInfo, QueryMsg};
use crate::query::{Cw20BalanceResponse, DumpStateResponse, GetItemResponse};
use crate::state::{
    Config, CONFIG, CW20_LIST, CW721_LIST, GOVERNANCE_MODULES, GOVERNANCE_MODULE_COUNT, ITEMS,
    PENDING_ITEM_INSTANTIATION_NAMES, VOTING_MODULE,
};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cw-governance";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const GOV_MODULE_REPLY_ID: u64 = 0;
const VOTE_MODULE_INSTANTIATE_REPLY_ID: u64 = 1;
const VOTE_MODULE_UPDATE_REPLY_ID: u64 = 2;

// Start at this ID since the items to instantiate on the instantiation
// of this contract can be arbitrarily long. Everything with a reply ID
// greater than or equal to this value will be considered an instantiated
// item to store in the items map.
const PENDING_ITEM_REPLY_ID_START: u64 = 100;
// The maximum number of items that can be instantiated when this
// contract is instantiated.
const MAX_ITEM_INSTANTIATIONS_ON_INSTANTIATE: u64 = u64::MAX - PENDING_ITEM_REPLY_ID_START;

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

    let vote_module_msg = msg
        .voting_module_instantiate_info
        .into_wasm_msg(env.contract.address.clone());
    let vote_module_msg: SubMsg<Empty> =
        SubMsg::reply_on_success(vote_module_msg, VOTE_MODULE_INSTANTIATE_REPLY_ID);

    let gov_module_msgs: Vec<SubMsg<Empty>> = msg
        .governance_modules_instantiate_info
        .into_iter()
        .map(|info| info.into_wasm_msg(env.contract.address.clone()))
        .map(|wasm| SubMsg::reply_on_success(wasm, GOV_MODULE_REPLY_ID))
        .collect();
    if gov_module_msgs.is_empty() {
        return Err(ContractError::NoGovernanceModule {});
    }

    GOVERNANCE_MODULE_COUNT.save(deps.storage, &(gov_module_msgs.len() as u64))?;

    // Add or instantiate items if any are present.
    let mut instantiate_item_msgs: Vec<SubMsg<Empty>> = vec![];
    if let Some(items) = msg.initial_items {
        if !items.is_empty() {
            if items.len() > MAX_ITEM_INSTANTIATIONS_ON_INSTANTIATE.try_into().unwrap() {
                return Err(ContractError::TooManyItems(
                    MAX_ITEM_INSTANTIATIONS_ON_INSTANTIATE,
                    items.len(),
                ));
            }

            for (idx, item) in items.into_iter().enumerate() {
                match item.info {
                    // Use existing address.
                    InitialItemInfo::Existing { address } => {
                        let addr = deps.api.addr_validate(&address)?;
                        ITEMS.save(deps.storage, item.name, &addr)?;
                    }
                    // Instantiate new contract and capture address on successful reply.
                    InitialItemInfo::Instantiate { info } => {
                        // Offset reply ID with index.
                        let reply_id = PENDING_ITEM_REPLY_ID_START + idx as u64;

                        // Create and add submessage.
                        let item_msg = info.into_wasm_msg(env.contract.address.clone());
                        let item_msg: SubMsg<Empty> = SubMsg::reply_on_success(item_msg, reply_id);
                        instantiate_item_msgs.push(item_msg);

                        // Store name in map for later retrieval if the contract instantiation succeeds.
                        PENDING_ITEM_INSTANTIATION_NAMES.save(
                            deps.storage,
                            reply_id,
                            &item.name,
                        )?;
                    }
                }
            }
        }
    }

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("sender", info.sender)
        .add_submessage(vote_module_msg)
        .add_submessages(gov_module_msgs)
        .add_submessages(instantiate_item_msgs))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::ExecuteProposalHook { msgs } => {
            execute_proposal_hook(deps.as_ref(), info.sender, msgs)
        }
        ExecuteMsg::UpdateConfig { config } => {
            execute_update_config(deps, env, info.sender, config)
        }
        ExecuteMsg::UpdateVotingModule { module } => {
            execute_update_voting_module(env, info.sender, module)
        }
        ExecuteMsg::UpdateGovernanceModules { to_add, to_remove } => {
            execute_update_governance_modules(deps, env, info.sender, to_add, to_remove)
        }
        ExecuteMsg::SetItem { key, addr } => execute_set_item(deps, env, info.sender, key, addr),
        ExecuteMsg::RemoveItem { key } => execute_remove_item(deps, env, info.sender, key),
        ExecuteMsg::Receive(_) => execute_receive_cw20(deps, info.sender),
        ExecuteMsg::ReceiveNft(_) => execute_receive_cw721(deps, info.sender),
        ExecuteMsg::UpdateCw20List { to_add, to_remove } => {
            execute_update_cw20_list(deps, env, info.sender, to_add, to_remove)
        }
        ExecuteMsg::UpdateCw721List { to_add, to_remove } => {
            execute_update_cw721_list(deps, env, info.sender, to_add, to_remove)
        }
    }
}

pub fn execute_proposal_hook(
    deps: Deps,
    sender: Addr,
    msgs: Vec<CosmosMsg<Empty>>,
) -> Result<Response, ContractError> {
    if !GOVERNANCE_MODULES.has(deps.storage, sender) {
        return Err(ContractError::Unauthorized {});
    }
    Ok(Response::default()
        .add_attribute("action", "execute_proposal_hook")
        .add_messages(msgs))
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
    // 'wasm._contract_address=governance&wasm.name=name'`.
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

pub fn execute_update_governance_modules(
    deps: DepsMut,
    env: Env,
    sender: Addr,
    to_add: Vec<ModuleInstantiateInfo>,
    to_remove: Vec<String>,
) -> Result<Response, ContractError> {
    if env.contract.address != sender {
        return Err(ContractError::Unauthorized {});
    }

    let module_count = GOVERNANCE_MODULE_COUNT.load(deps.storage)?;

    // Some safe maths.
    let new_total = module_count
        .checked_add(to_add.len() as u64)
        .ok_or(ContractError::Overflow {})?
        .checked_sub(to_remove.len() as u64)
        .ok_or(ContractError::Overflow {})?;
    if new_total == 0 {
        return Err(ContractError::NoGovernanceModule {});
    }
    GOVERNANCE_MODULE_COUNT.save(deps.storage, &new_total)?;

    for addr in to_remove {
        let addr = deps.api.addr_validate(&addr)?;
        GOVERNANCE_MODULES.remove(deps.storage, addr);
    }

    let to_add: Vec<SubMsg<Empty>> = to_add
        .into_iter()
        .map(|info| info.into_wasm_msg(env.contract.address.clone()))
        .map(|wasm| SubMsg::reply_on_success(wasm, GOV_MODULE_REPLY_ID))
        .collect();

    Ok(Response::default()
        .add_attribute("action", "execute_update_governance_modules")
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
    addr: String,
) -> Result<Response, ContractError> {
    if env.contract.address != sender {
        return Err(ContractError::Unauthorized {});
    }

    let addr = deps.api.addr_validate(&addr)?;
    ITEMS.save(deps.storage, key.clone(), &addr)?;
    Ok(Response::default()
        .add_attribute("action", "execute_set_item")
        .add_attribute("key", key)
        .add_attribute("addr", addr))
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
        QueryMsg::Config {} => query_config(deps),
        QueryMsg::VotingModule {} => query_voting_module(deps),
        QueryMsg::GovernanceModules { start_at, limit } => {
            query_governance_modules(deps, start_at, limit)
        }
        QueryMsg::DumpState {} => query_dump_state(deps),
        QueryMsg::VotingPowerAtHeight { address, height } => {
            query_voting_power_at_height(deps, address, height)
        }
        QueryMsg::TotalPowerAtHeight { height } => query_total_power_at_height(deps, height),
        QueryMsg::GetItem { key } => query_get_item(deps, key),
        QueryMsg::ListItems { start_at, limit } => query_list_items(deps, start_at, limit),
        QueryMsg::Info {} => query_info(deps),
        QueryMsg::Cw20TokenList { start_at, limit } => query_cw20_list(deps, start_at, limit),
        QueryMsg::Cw721TokenList { start_at, limit } => query_cw721_list(deps, start_at, limit),
        QueryMsg::Cw20Balances { start_at, limit } => {
            query_cw20_balances(deps, env, start_at, limit)
        }
    }
}

pub fn query_config(deps: Deps) -> StdResult<Binary> {
    let config = CONFIG.load(deps.storage)?;
    to_binary(&config)
}

pub fn query_voting_module(deps: Deps) -> StdResult<Binary> {
    let voting_module = VOTING_MODULE.load(deps.storage)?;
    to_binary(&voting_module)
}

pub fn query_governance_modules(
    deps: Deps,
    start_at: Option<String>,
    limit: Option<u64>,
) -> StdResult<Binary> {
    let start_at = start_at.map(|a| deps.api.addr_validate(&a)).transpose()?;
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
    // governance modules by looking at past transactions on chain.
    let modules = GOVERNANCE_MODULES.keys(
        deps.storage,
        start_at.map(Bound::inclusive),
        None,
        cosmwasm_std::Order::Descending,
    );
    let modules: Vec<Addr> = match limit {
        Some(limit) => modules
            .take(limit as usize)
            .collect::<Result<Vec<Addr>, _>>()?,
        None => modules.collect::<Result<Vec<Addr>, _>>()?,
    };
    to_binary(&modules)
}

pub fn query_dump_state(deps: Deps) -> StdResult<Binary> {
    let config = CONFIG.load(deps.storage)?;
    let voting_module = VOTING_MODULE.load(deps.storage)?;
    let governance_modules = GOVERNANCE_MODULES
        .keys(deps.storage, None, None, cosmwasm_std::Order::Descending)
        .collect::<Result<Vec<Addr>, _>>()?;
    let version = get_contract_version(deps.storage)?;
    to_binary(&DumpStateResponse {
        config,
        version,
        governance_modules,
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
    limit: Option<u64>,
) -> StdResult<Binary> {
    let items = ITEMS.keys(
        deps.storage,
        start_at.map(Bound::inclusive),
        None,
        cosmwasm_std::Order::Descending,
    );
    let items = match limit {
        Some(limit) => items
            .take(limit as usize)
            .collect::<Result<Vec<String>, _>>()?,
        None => items.collect::<Result<Vec<String>, _>>()?,
    };

    to_binary(&items)
}

// Can't be generic over key type. Otherwise, we could use this in
// `query_list_items` as well.
// <https://github.com/CosmWasm/cw-plus/issues/691>
fn list_addr_keys<V>(
    deps: Deps,
    map: Map<Addr, V>,
    start_at: Option<Addr>,
    limit: Option<u64>,
) -> StdResult<Vec<Addr>>
where
    V: serde::de::DeserializeOwned + serde::Serialize,
{
    let items = map.keys(
        deps.storage,
        start_at.map(Bound::inclusive),
        None,
        cosmwasm_std::Order::Descending,
    );
    match limit {
        Some(limit) => Ok(items
            .take(limit as usize)
            .collect::<Result<Vec<Addr>, _>>()?),
        None => Ok(items.collect::<Result<Vec<Addr>, _>>()?),
    }
}

pub fn query_cw20_list(
    deps: Deps,
    start_at: Option<String>,
    limit: Option<u64>,
) -> StdResult<Binary> {
    to_binary(&list_addr_keys(
        deps,
        CW20_LIST,
        start_at.map(|s| deps.api.addr_validate(&s)).transpose()?,
        limit,
    )?)
}

pub fn query_cw721_list(
    deps: Deps,
    start_at: Option<String>,
    limit: Option<u64>,
) -> StdResult<Binary> {
    to_binary(&list_addr_keys(
        deps,
        CW721_LIST,
        start_at.map(|s| deps.api.addr_validate(&s)).transpose()?,
        limit,
    )?)
}

pub fn query_cw20_balances(
    deps: Deps,
    env: Env,
    start_at: Option<String>,
    limit: Option<u64>,
) -> StdResult<Binary> {
    let addrs = list_addr_keys(
        deps,
        CW20_LIST,
        start_at.map(|a| deps.api.addr_validate(&a)).transpose()?,
        limit,
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
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        GOV_MODULE_REPLY_ID => {
            let res = parse_reply_instantiate_data(msg)?;
            let gov_module_addr = deps.api.addr_validate(&res.contract_address)?;
            GOVERNANCE_MODULES.save(deps.storage, gov_module_addr, &Empty {})?;

            Ok(Response::default().add_attribute("gov_module".to_string(), res.contract_address))
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
        reply_id if reply_id >= PENDING_ITEM_REPLY_ID_START => {
            // Retrieve the name using the ID. If it doesn't exist,
            // we didn't expect this reply or it was a redundant execution.
            let item_name = PENDING_ITEM_INSTANTIATION_NAMES.load(deps.storage, reply_id)?;

            let res = parse_reply_instantiate_data(msg)?;
            let item_addr = deps.api.addr_validate(&res.contract_address)?;

            ITEMS.save(deps.storage, item_name, &item_addr)?;
            // Remove from pending map since we now have the contract address.
            PENDING_ITEM_INSTANTIATION_NAMES.remove(deps.storage, reply_id);

            Ok(Response::default().add_attribute("item", item_addr))
        }
        _ => Err(ContractError::UnknownReplyID {}),
    }
}
