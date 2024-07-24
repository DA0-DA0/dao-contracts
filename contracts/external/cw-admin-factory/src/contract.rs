#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdResult, SubMsg,
    WasmMsg,
};

use cw2::set_contract_version;
use cw_utils::parse_reply_instantiate_data;

use crate::error::ContractError;
use crate::msg::{AdminResponse, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use crate::state::{ADMIN, EXPECT};

pub(crate) const CONTRACT_NAME: &str = "crates.io:cw-admin-factory";
pub(crate) const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const INSTANTIATE_CONTRACT_REPLY_ID: u64 = 0;
pub const INSTANTIATE2_CONTRACT_REPLY_ID: u64 = 2;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let admin = msg.admin.map(|a| deps.api.addr_validate(&a)).transpose()?;
    ADMIN.save(deps.storage, &admin)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("creator", info.sender))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::InstantiateContractWithSelfAdmin {
            instantiate_msg: msg,
            code_id,
            label,
        } => instantiate_contract(deps, env, info, msg, code_id, label),
        ExecuteMsg::Instantiate2ContractWithSelfAdmin {
            instantiate_msg: msg,
            code_id,
            label,
            salt,
            expect,
        } => instantiate2_contract(deps, env, info, msg, code_id, label, salt, expect),
    }
}

pub fn instantiate_contract(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    instantiate_msg: Binary,
    code_id: u64,
    label: String,
) -> Result<Response, ContractError> {
    // If admin set, require the sender to be the admin.
    if let Some(admin) = ADMIN.load(deps.storage)? {
        if admin != info.sender {
            return Err(ContractError::Unauthorized {});
        }
    }

    // Instantiate the specified contract with factory as the admin.
    let instantiate = WasmMsg::Instantiate {
        admin: Some(env.contract.address.to_string()),
        code_id,
        msg: instantiate_msg,
        funds: info.funds,
        label,
    };

    let msg = SubMsg::reply_on_success(instantiate, INSTANTIATE_CONTRACT_REPLY_ID);
    Ok(Response::default()
        .add_attribute("action", "instantiate_contract_with_self_admin")
        .add_submessage(msg))
}

#[allow(clippy::too_many_arguments)]
pub fn instantiate2_contract(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    instantiate_msg: Binary,
    code_id: u64,
    label: String,
    salt: Binary,
    expect: Option<String>,
) -> Result<Response, ContractError> {
    // If admin set, require the sender to be the admin.
    if let Some(admin) = ADMIN.load(deps.storage)? {
        if admin != info.sender {
            return Err(ContractError::Unauthorized {});
        }
    }

    if let Some(expect) = expect {
        let expect = deps.api.addr_validate(&expect)?;
        EXPECT.save(deps.storage, &expect)?;
    }

    // Instantiate the specified contract with factory as the admin.
    let instantiate = WasmMsg::Instantiate2 {
        admin: Some(env.contract.address.to_string()),
        code_id,
        msg: instantiate_msg,
        funds: info.funds,
        label,
        salt,
    };

    let msg = SubMsg::reply_on_success(instantiate, INSTANTIATE2_CONTRACT_REPLY_ID);
    Ok(Response::default()
        .add_attribute("action", "instantiate2_contract_with_self_admin")
        .add_submessage(msg))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Admin {} => Ok(to_json_binary(&AdminResponse {
            admin: ADMIN.load(deps.storage)?,
        })?),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    let msg_id = msg.id;
    match msg_id {
        INSTANTIATE_CONTRACT_REPLY_ID | INSTANTIATE2_CONTRACT_REPLY_ID => {
            let res = parse_reply_instantiate_data(msg)?;
            let contract_addr = deps.api.addr_validate(&res.contract_address)?;

            if msg_id == INSTANTIATE2_CONTRACT_REPLY_ID {
                // If saved an expected address, verify it matches and clear it.
                let expect = EXPECT.may_load(deps.storage)?;
                if let Some(expect) = expect {
                    EXPECT.remove(deps.storage);
                    if contract_addr != expect {
                        return Err(ContractError::UnexpectedContractAddress {
                            expected: expect.to_string(),
                            actual: contract_addr.to_string(),
                        });
                    }
                }
            }

            // Make the contract its own admin.
            let msg = WasmMsg::UpdateAdmin {
                contract_addr: contract_addr.to_string(),
                admin: contract_addr.to_string(),
            };

            Ok(Response::default()
                .add_attribute("set contract admin as itself", contract_addr)
                .add_message(msg))
        }
        _ => Err(ContractError::UnknownReplyID {}),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    // Set contract to version to latest
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}
