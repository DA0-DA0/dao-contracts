#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Order, Reply, Response, StdResult, SubMsg,
    WasmMsg,
};

use cw2::set_contract_version;
use cw_payroll::msg::InstantiateMsg as PayrollInstantiateMsg;
use cw_storage_plus::Bound;
use cw_utils::parse_reply_instantiate_data;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use crate::state::{vesting_contracts, VestingContract, TMP_CONTRACT_INFO};

pub(crate) const CONTRACT_NAME: &str = "crates.io:cw-payroll-factory";
pub(crate) const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const INSTANTIATE_CONTRACT_REPLY_ID: u64 = 0;
pub const DEFAULT_LIMIT: u32 = 10;
pub const MAX_LIMIT: u32 = 50;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
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
        ExecuteMsg::InstantiatePayrollContract {
            instantiate_msg: msg,
            code_id,
            label,
        } => instantiate_contract(env, deps, info, msg, code_id, label),
    }
}

pub fn instantiate_contract(
    _env: Env,
    deps: DepsMut,
    info: MessageInfo,
    instantiate_msg: PayrollInstantiateMsg,
    code_id: u64,
    label: String,
) -> Result<Response, ContractError> {
    // Instantiate the specified contract with owner as the admin.
    let instantiate = WasmMsg::Instantiate {
        admin: instantiate_msg.owner.clone(),
        code_id,
        msg: to_binary(&instantiate_msg)?,
        funds: info.funds,
        label,
    };

    let msg = SubMsg::reply_on_success(instantiate, INSTANTIATE_CONTRACT_REPLY_ID);

    let instantiator = deps
        .api
        .addr_validate(&instantiate_msg.owner.unwrap_or(info.sender.to_string()))?;
    let recipient = deps.api.addr_validate(&instantiate_msg.params.recipient)?;

    // Save tmp contract info for use in reply
    TMP_CONTRACT_INFO.save(deps.storage, &(recipient, instantiator))?;

    Ok(Response::default()
        .add_attribute("action", "instantiate_cw_payroll")
        .add_submessage(msg))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::ListVestingContracts { start_after, limit } => {
            let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
            let start = start_after.as_deref().map(Bound::exclusive);

            let res: Vec<VestingContract> = vesting_contracts()
                .range(deps.storage, start, None, Order::Ascending)
                .take(limit)
                .map(|vc| Ok::<VestingContract, ContractError>(vc?.1))
                .flatten()
                .collect();

            Ok(to_binary(&res)?)
        }
        QueryMsg::ListVestingContractsByInstantiator {
            instantiator,
            start_after,
            limit,
        } => {
            let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
            let start = start_after.map(Bound::<String>::exclusive);

            // Validate owner address
            deps.api.addr_validate(&instantiator)?;

            let res: Vec<VestingContract> = vesting_contracts()
                .idx
                .owner
                .prefix(instantiator)
                .range(deps.storage, start, None, Order::Ascending)
                .take(limit)
                .map(|vc| Ok::<VestingContract, ContractError>(vc?.1))
                .flatten()
                .collect();

            Ok(to_binary(&res)?)
        }
        QueryMsg::ListVestingContractsByRecipient {
            recipient,
            start_after,
            limit,
        } => {
            let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
            let start = start_after.map(Bound::<String>::exclusive);

            // Validate recipient address
            deps.api.addr_validate(&recipient)?;

            let res: Vec<VestingContract> = vesting_contracts()
                .idx
                .recipient
                .prefix(recipient)
                .range(deps.storage, start, None, Order::Ascending)
                .take(limit)
                .map(|vc| Ok::<VestingContract, ContractError>(vc?.1))
                .flatten()
                .collect();

            Ok(to_binary(&res)?)
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        INSTANTIATE_CONTRACT_REPLY_ID => {
            let res = parse_reply_instantiate_data(msg)?;
            let contract_addr = deps.api.addr_validate(&res.contract_address)?;

            let (recipient, instantiator) = TMP_CONTRACT_INFO.load(deps.storage)?;

            // Save vesting contract payment info
            vesting_contracts().save(
                deps.storage,
                contract_addr.as_str().clone(),
                &VestingContract {
                    owner: instantiator.to_string(),
                    recipient: recipient.to_string(),
                    contract: contract_addr.clone().to_string(),
                },
            )?;

            // Clear tmp contract info
            TMP_CONTRACT_INFO.remove(deps.storage);

            Ok(Response::default().add_attribute("new_payroll_contract", contract_addr))
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
