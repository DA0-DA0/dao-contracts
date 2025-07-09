#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdError,
    StdResult,
};
use cw_jsonfilter::{CwJsonFilter, FilterResult};

use cw2::set_contract_version;
use cw_ownable::initialize_owner;
use cw_utils::{nonpayable, parse_reply_instantiate_data};
use dao_interface::proposal::InfoResponse;
use dao_interface::state::ModuleUpdate;

use crate::error::ContractError;
use crate::msg::{
    ExecuteMsg, FilterResponse, InstantiateMsg, MigrateMsg, ProtobufRegistryResponse, QueryMsg,
};
use crate::state::PROTOBUF_REGISTRY;

pub(crate) const CONTRACT_NAME: &str = "crates.io:cw-filter";
pub(crate) const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const INSTANTIATE_PROTOBUF_REGISTRY_REPLY_ID: u64 = 1;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Default the owner to the sender.
    let owner = msg.owner.map_or_else(
        || Ok(info.sender.clone()),
        |owner| deps.api.addr_validate(&owner),
    )?;
    initialize_owner(deps.storage, deps.api, Some(owner.as_str()))?;

    // Initialize protobuf registry by either creating it or using an existing
    // one. Use this contract's owner as the admin.
    let protobuf_registry_message = msg.protobuf_registry.map_or_else(
        || Ok(vec![]),
        |protobuf_registry| {
            protobuf_registry.update(
                deps.branch(),
                &PROTOBUF_REGISTRY,
                INSTANTIATE_PROTOBUF_REGISTRY_REPLY_ID,
                &owner,
            )
        },
    )?;

    Ok(Response::new()
        .add_submessages(protobuf_registry_message)
        .add_attribute("action", "instantiate")
        .add_attribute("creator", info.sender.as_str())
        .add_attribute("owner", owner.as_str()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    match msg {
        ExecuteMsg::UpdateOwnership(action) => execute_update_ownership(deps, info, env, action),
        ExecuteMsg::UpdateProtobufRegistry { protobuf_registry } => {
            execute_update_protobuf_registry(deps, info, protobuf_registry)
        }
    }
}

fn execute_update_ownership(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    action: cw_ownable::Action,
) -> Result<Response, ContractError> {
    let ownership = cw_ownable::update_ownership(deps, &env.block, &info.sender, action)?;
    Ok(Response::new()
        .add_attribute("action", "update_ownership")
        .add_attributes(ownership.into_attributes()))
}

fn execute_update_protobuf_registry(
    deps: DepsMut,
    info: MessageInfo,
    protobuf_registry: Option<ModuleUpdate>,
) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    let protobuf_registry_message = match protobuf_registry {
        Some(protobuf_registry) => protobuf_registry.update(
            deps,
            &PROTOBUF_REGISTRY,
            INSTANTIATE_PROTOBUF_REGISTRY_REPLY_ID,
            &info.sender,
        ),
        None => {
            PROTOBUF_REGISTRY.remove(deps.storage);
            Ok(vec![])
        }
    }?;

    Ok(Response::new()
        .add_submessages(protobuf_registry_message)
        .add_attribute("action", "update_protobuf_registry"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Ownership {} => to_json_binary(&cw_ownable::get_ownership(deps.storage)?),
        QueryMsg::Info {} => to_json_binary(&query_info(deps)?),
        QueryMsg::ProtobufRegistry {} => to_json_binary(&query_protobuf_registry(deps)?),
        QueryMsg::Filter { filter, msg } => to_json_binary(&query_filter(deps, filter, msg)?),
    }
}

fn query_info(deps: Deps) -> StdResult<InfoResponse> {
    let info = cw2::get_contract_version(deps.storage)?;
    Ok(InfoResponse { info })
}

fn query_protobuf_registry(deps: Deps) -> StdResult<ProtobufRegistryResponse> {
    let protobuf_registry = PROTOBUF_REGISTRY.may_load(deps.storage)?;
    Ok(ProtobufRegistryResponse { protobuf_registry })
}

fn query_filter(
    deps: Deps,
    filter: serde_json::Value,
    msg: CosmosMsg,
) -> StdResult<FilterResponse> {
    let protobuf_registry = PROTOBUF_REGISTRY.may_load(deps.storage)?;

    let decode_protobuf = protobuf_registry.map(
        |protobuf_registry| -> Box<dyn Fn(String, Vec<u8>) -> Result<serde_json::Value, String>> {
            Box::new(move |message_name, value| {
                deps.querier
                    .query_wasm_smart::<cw_protobuf_registry::msg::DecodeResponse>(
                        &protobuf_registry,
                        &cw_protobuf_registry::msg::QueryMsg::Decode {
                            message_name: message_name.to_string(),
                            value,
                        },
                    )
                    .map(|r| r.value)
                    .map_err(|e| e.to_string())
            })
        },
    );

    let msg_value = serde_json::to_value(msg).map_err(|e| {
        StdError::generic_err(ContractError::JsonSerialization { err: e.to_string() }.to_string())
    })?;

    let result = CwJsonFilter::new(decode_protobuf).matches(&filter, &msg_value);

    let response = match result {
        FilterResult::Pass => FilterResponse::Pass {},
        FilterResult::Fail(error) => FilterResponse::Fail {
            reason: error.to_string(),
        },
        FilterResult::Fatal(error) => {
            let reason = error.to_string();
            if reason.contains("protobuf decoder not provided") {
                FilterResponse::Fatal {
                    reason: ContractError::MissingProtobufRegistry {}.to_string(),
                }
            } else {
                FilterResponse::Fatal { reason }
            }
        }
    };

    Ok(response)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    // Set contract to version to latest
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        INSTANTIATE_PROTOBUF_REGISTRY_REPLY_ID => {
            let res = parse_reply_instantiate_data(msg)?;
            let addr = deps.api.addr_validate(&res.contract_address)?;

            PROTOBUF_REGISTRY.save(deps.storage, &addr)?;

            Ok(Response::default().add_attribute("protobuf_registry", addr))
        }

        _ => Err(ContractError::UnknownReplyID { id: msg.id }),
    }
}
