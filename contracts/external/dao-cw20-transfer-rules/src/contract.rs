#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Order, Response, StdResult,
};
use cw2::set_contract_version;
use cw20_hooks::hooks::Cw20HookMsg;
use cw_storage_plus::Bound;
use cw_utils::maybe_addr;
use dao_interface::msg::QueryMsg as DaoQueryMsg;
use dao_interface::voting::VotingPowerAtHeightResponse;

use crate::error::ContractError;
use crate::msg::{
    AllowanceEntry, AllowanceResponse, AllowanceUpdate, ConfigResponse, ExecuteMsg, InstantiateMsg,
    ListAllowancesResponse, MigrateMsg, QueryMsg,
};
use crate::state::{Allowance, Config, ALLOWANCES, CONFIG};

pub(crate) const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
pub(crate) const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

// Settings for query pagination
const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    cw_ownable::initialize_owner(deps.storage, deps.api, msg.owner.as_deref())?;

    // Validate DAO address.
    let dao = deps.api.addr_validate(&msg.dao)?;
    // Query DAO for voting power of sender to verify it responds to the query.
    let _: VotingPowerAtHeightResponse = deps
        .querier
        .query_wasm_smart(
            dao.clone(),
            &DaoQueryMsg::VotingPowerAtHeight {
                address: info.sender.to_string(),
                height: None,
            },
        )
        .map_err(|error| ContractError::InvalidDao { error })?;

    // Save config.
    CONFIG.save(
        deps.storage,
        &Config {
            dao: dao.clone(),
            member_allowance: msg.member_allowance.unwrap_or(Allowance::None),
        },
    )?;

    // Initialize allowances if provided.
    if let Some(allowances) = msg.allowances {
        for entry in allowances {
            if entry.allowance != Allowance::None {
                ALLOWANCES.save(
                    deps.storage,
                    &deps.api.addr_validate(&entry.address)?,
                    &entry.allowance,
                )?;
            }
        }
    }

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", msg.owner.unwrap_or("none".to_string()))
        .add_attribute("dao", dao))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Cw20Hook(msg) => execute_check_transfer(deps, env, info, msg),
        ExecuteMsg::UpdateAllowances { set, remove } => {
            execute_update_allowances(deps, info, set, remove)
        }
        ExecuteMsg::UpdateConfig {
            dao,
            member_allowance,
        } => execute_update_config(deps, info, dao, member_allowance),
        ExecuteMsg::UpdateOwnership(action) => execute_update_owner(deps, info, env, action),
    }
}

pub fn execute_check_transfer(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: Cw20HookMsg,
) -> Result<Response, ContractError> {
    let (sender, recipient) = match msg {
        Cw20HookMsg::Transfer {
            sender, recipient, ..
        } => (
            deps.api.addr_validate(&sender)?,
            deps.api.addr_validate(&recipient)?,
        ),
        Cw20HookMsg::Send {
            sender, contract, ..
        } => (
            deps.api.addr_validate(&sender)?,
            deps.api.addr_validate(&contract)?,
        ),
    };

    // Check if sender can send.
    let (sender_allowed, send_anywhere) = is_allowed(deps.as_ref(), sender.to_string(), true)?;

    // Check if recipient can receive.
    let (recipient_allowed, receive_anywhere) =
        is_allowed(deps.as_ref(), recipient.to_string(), false)?;

    // Unauthorized if sender cannot send AND recipient cannot receive from any
    // sender.
    if !sender_allowed && !receive_anywhere {
        return Err(ContractError::UnauthorizedSender {});
    }

    // Unauthorized if recipient cannot receive AND sender cannot send to any
    // recipient.
    if !recipient_allowed && !send_anywhere {
        return Err(ContractError::UnauthorizedRecipient {});
    }

    Ok(Response::default())
}

// Return whether or not the address can send/receive and if they can do so
// to/from anywhere.
pub fn is_allowed(
    deps: Deps,
    address: String,
    // If true, check if allowed to send. If false, check if allowed to receive.
    sending: bool,
) -> StdResult<(bool, bool)> {
    let allowance = query_allowance(deps, address)?.allowance;
    Ok(match allowance {
        Allowance::None => (false, false),
        Allowance::Send => (sending, false),
        Allowance::SendAnywhere => (sending, true),
        Allowance::Receive => (!sending, false),
        Allowance::ReceiveAnywhere => (!sending, true),
        Allowance::SendAndReceive => (true, false),
        Allowance::SendAndReceiveAnywhere => (true, true),
    })
}

pub fn execute_update_allowances(
    deps: DepsMut,
    info: MessageInfo,
    set: Vec<AllowanceUpdate>,
    remove: Vec<String>,
) -> Result<Response, ContractError> {
    // Check if sender is the owner.
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    // Update address allowances.
    for entry in set {
        ALLOWANCES.save(
            deps.storage,
            &deps.api.addr_validate(&entry.address)?,
            &entry.allowance,
        )?;
    }

    // Remove address allowances.
    for address in remove {
        ALLOWANCES.remove(deps.storage, &deps.api.addr_validate(&address)?);
    }

    Ok(Response::default().add_attribute("action", "update_allowances"))
}

pub fn execute_update_config(
    deps: DepsMut,
    info: MessageInfo,
    dao: Option<String>,
    member_allowance: Option<Allowance>,
) -> Result<Response, ContractError> {
    // Check if sender is the owner.
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    let mut config = CONFIG.load(deps.storage)?;

    // Update if provided.
    if let Some(dao) = dao {
        config.dao = deps.api.addr_validate(&dao)?;
    }

    // Update if provided.
    if let Some(member_allowance) = member_allowance {
        config.member_allowance = member_allowance;
    }

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::default().add_attribute("action", "update_config"))
}

pub fn execute_update_owner(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    action: cw_ownable::Action,
) -> Result<Response, ContractError> {
    let ownership = cw_ownable::update_ownership(deps, &env.block, &info.sender, action)?;
    Ok(Response::default().add_attributes(ownership.into_attributes()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_json_binary(&ConfigResponse {
            config: CONFIG.load(deps.storage)?,
        }),
        QueryMsg::ListAllowances { start_after, limit } => {
            query_list_allowances(deps, start_after, limit)
        }
        QueryMsg::Allowance { address } => to_json_binary(&query_allowance(deps, address)?),
        QueryMsg::Info {} => query_info(deps),
        QueryMsg::Ownership {} => to_json_binary(&cw_ownable::get_ownership(deps.storage)?),
    }
}

pub fn query_list_allowances(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Binary> {
    let addr = maybe_addr(deps.api, start_after)?;
    let start = addr.as_ref().map(Bound::exclusive);

    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let allowances = ALLOWANCES
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (address, allowance) = item?;
            Ok(AllowanceEntry { address, allowance })
        })
        .collect::<StdResult<Vec<AllowanceEntry>>>()?;

    to_json_binary(&ListAllowancesResponse { allowances })
}

pub fn query_allowance(deps: Deps, address: String) -> StdResult<AllowanceResponse> {
    let mut allowance = ALLOWANCES.may_load(deps.storage, &deps.api.addr_validate(&address)?)?;
    let mut is_member_allowance = false;

    // If no allowance found, use member allowance if the address is a member.
    if allowance.is_none() {
        let config = CONFIG.load(deps.storage)?;

        // Check if address has voting power in the DAO.
        let voting_power: VotingPowerAtHeightResponse = deps.querier.query_wasm_smart(
            config.dao,
            &DaoQueryMsg::VotingPowerAtHeight {
                address: address.to_string(),
                height: None,
            },
        )?;

        // If member, use member allowance.
        if !voting_power.power.is_zero() {
            allowance = Some(config.member_allowance);
            is_member_allowance = true;
        }
    }

    Ok(AllowanceResponse {
        allowance: allowance.unwrap_or(Allowance::None),
        is_member_allowance,
    })
}

pub fn query_info(deps: Deps) -> StdResult<Binary> {
    let info = cw2::get_contract_version(deps.storage)?;
    to_json_binary(&dao_interface::voting::InfoResponse { info })
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    // Set contract to version to latest
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}
