use std::usize;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, CosmosMsg, Deps, DepsMut, Empty, Env, MessageInfo, Reply, Response,
    StdResult, SubMsg,
};
use cw2::{get_contract_version, set_contract_version};
use cw_storage_plus::Bound;
use cw_utils::parse_reply_instantiate_data;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, ModuleInstantiateInfo, QueryMsg};
use crate::query::DumpStateResponse;
use crate::state::{Config, CONFIG, GOVERNANCE_MODULES, GOVERNANCE_MODULE_COUNT, VOTING_MODULE};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cw-governance";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const GOV_MODULE_REPLY_ID: u64 = 0;
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

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("sender", info.sender)
        .add_submessage(vote_module_msg)
        .add_submessages(gov_module_msgs))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::ExecuteProposalHook { .. } => {
            if !GOVERNANCE_MODULES.has(deps.storage, info.sender.clone()) {
                return Err(ContractError::Unauthorized {});
            }
        }
        ExecuteMsg::UpdateConfig { .. }
        | ExecuteMsg::UpdateVotingModule { .. }
        | ExecuteMsg::UpdateGovernanceModules { .. } => {
            if info.sender != env.contract.address {
                return Err(ContractError::Unauthorized {});
            }
        }
    }

    let response = match msg {
        ExecuteMsg::ExecuteProposalHook { msgs } => execute_proposal_hook(msgs),
        ExecuteMsg::UpdateConfig { config } => execute_update_config(deps, config),
        ExecuteMsg::UpdateVotingModule { module } => execute_update_voting_module(env, module),
        ExecuteMsg::UpdateGovernanceModules { to_add, to_remove } => {
            execute_update_governance_modules(deps, env, to_add, to_remove)
        }
    }?;

    Ok(response.add_attribute("sender", info.sender))
}

pub fn execute_proposal_hook(msgs: Vec<CosmosMsg<Empty>>) -> Result<Response, ContractError> {
    Ok(Response::default()
        .add_attribute("action", "execute_proposal_hook")
        .add_messages(msgs))
}

pub fn execute_update_config(deps: DepsMut, config: Config) -> Result<Response, ContractError> {
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
    module: ModuleInstantiateInfo,
) -> Result<Response, ContractError> {
    let wasm = module.into_wasm_msg(env.contract.address);
    let submessage = SubMsg::reply_on_success(wasm, VOTE_MODULE_UPDATE_REPLY_ID);

    Ok(Response::default()
        .add_attribute("action", "execute_update_voting_module")
        .add_submessage(submessage))
}

pub fn execute_update_governance_modules(
    deps: DepsMut,
    env: Env,
    to_add: Vec<ModuleInstantiateInfo>,
    to_remove: Vec<String>,
) -> Result<Response, ContractError> {
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

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => query_config(deps),
        QueryMsg::VotingModule {} => query_voting_module(deps),
        QueryMsg::GovernanceModules { start_at, limit } => {
            query_governance_modules(deps, start_at, limit)
        }
        QueryMsg::DumpState {} => query_dump_state(deps),
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
    // governance modules by looking at past transactions onchain.
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
        _ => Err(ContractError::UnknownReplyID {}),
    }
}
