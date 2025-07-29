use cosmwasm_std::{to_json_binary, Addr, DepsMut, Env, StdResult, SubMsg};
use dao_interface::state::{Admin, ModuleInstantiateInfo, ModuleUpdate};

use crate::{
    contract::{INSTANTIATE_FILTER_REPLY_ID, INSTANTIATE_PROTOBUF_REGISTRY_REPLY_ID}, state::{ENABLED, NEXT_ID}, ContractError
};

/// Ensure the RBAM system is enabled.
pub fn ensure_enabled(deps: DepsMut) -> Result<(), ContractError> {
    if !ENABLED.load(deps.storage)? {
        return Err(ContractError::SystemDisabled {});
    }
    Ok(())
}

/// Gets the next ID and increments state by first incrementing in state and
/// then decrementing the updated return value.
pub fn get_next_id(deps: DepsMut) -> StdResult<u64> {
    NEXT_ID
        // Increment the ID in state
        .update(deps.storage, |id| Ok(id + 1))
        // Decrement the new ID to get the previous ID
        .map(|id| id - 1)
}

fn get_module_label(env: &Env, suffix: &str) -> String {
    let last6 = env.contract.address.to_string().chars().rev().take(6).collect::<String>();
    format!("rbam-{}-{}", last6, suffix)
}

pub fn submsg_instantiate_registry(
    env: &Env,
    owner: String,
    code_id: u64,
    salt: Option<cosmwasm_std::Binary>,
) -> Result<SubMsg, ContractError> {
    let submsg = SubMsg::reply_on_success(
        ModuleInstantiateInfo {
            code_id,
            // Set this RBAM as owner so we can prepare protobuf messages.
            msg: to_json_binary(&cw_protobuf_registry::msg::InstantiateMsg {
                owner: Some(env.contract.address.to_string()),
            })?,
            // Set RBAM's owner as the admin so they can upgrade it.
            admin: Some(Admin::Address {
                addr: owner,
            }),
            salt,
            funds: None,
            label: get_module_label(&env, "protobuf-registry")
        }
        .into_cosmos_msg(""),
        INSTANTIATE_PROTOBUF_REGISTRY_REPLY_ID,
    );

    Ok(submsg)
}


pub fn submsg_instantiate_filter(
    env: &Env,
    owner: String,
    code_id: u64,
    salt: Option<cosmwasm_std::Binary>,
    protobuf_registry: Option<Addr>,
) -> Result<SubMsg, ContractError> {
    let protobuf_registry = protobuf_registry.map(|addr| ModuleUpdate::Existing { address: addr.to_string() });

    let module_init_msg = ModuleInstantiateInfo {
        code_id,
        // Set RBAM's owner as owner.
        msg: to_json_binary(&cw_filter::msg::InstantiateMsg {
            owner: Some(owner.to_string()),
            protobuf_registry,
        })?,
        // Set RBAM's owner as the admin so they can upgrade it.
        admin: Some(Admin::Address {
            addr: owner.to_string(),
        }),
        salt,
        funds: None,
        label: get_module_label(&env, "filter"),
    }
    .into_cosmos_msg("");

    let submsg = SubMsg::reply_on_success(
        module_init_msg,
        INSTANTIATE_FILTER_REPLY_ID,
    );

    Ok(submsg)
}
