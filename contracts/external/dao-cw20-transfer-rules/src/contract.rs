#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Order, Response, StdResult,
};
use cw2::set_contract_version;
use cw_storage_plus::Bound;
use dao_cw20::Cw20HookMsg;
use dao_interface::msg::QueryMsg as DaoQueryMsg;
use dao_interface::voting::{Query as VotingQueryMsg, VotingPowerAtHeightResponse};

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use crate::state::{ALLOWLIST, DAO, DAO_VOTING_MODULE};

pub(crate) const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
pub(crate) const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const INSTANTIATE_CONTRACT_REPLY_ID: u64 = 0;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Validate DAO address and save it
    let dao = deps.api.addr_validate(&msg.dao)?;
    DAO.save(deps.storage, &dao)?;

    // Query DAO voting module to ensure it is a valid dao
    let voting_module: Addr = deps
        .querier
        .query_wasm_smart(dao.clone(), &DaoQueryMsg::VotingModule {})?;

    // Validate DAO address and save it
    DAO_VOTING_MODULE.save(deps.storage, &voting_module)?;

    // Initialize allowlist if provided
    if let Some(allowlist) = msg.allowlist {
        for address in allowlist {
            ALLOWLIST.save(deps.storage, &deps.api.addr_validate(&address)?, &())?;
        }
    }

    // Set DAO as the contract owner
    cw_ownable::initialize_owner(deps.storage, deps.api, Some(dao.as_str()))?;

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
        ExecuteMsg::Cw20Hook(msg) => execute_check_transfer(deps, env, info, msg),
        ExecuteMsg::UpdateAllowlist { add, remove } => {
            execute_update_allowlist(deps, info, add, remove)
        }
    }
}

pub fn execute_check_transfer(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: Cw20HookMsg,
) -> Result<Response, ContractError> {
    // TODO check this comes from the correct cw20 contract?

    let recipient = match msg {
        Cw20HookMsg::Transfer { recipient, .. } => deps.api.addr_validate(&recipient)?,
        Cw20HookMsg::Send { contract, .. } => deps.api.addr_validate(&contract)?,
    };

    let dao_voting_module = DAO_VOTING_MODULE.load(deps.storage)?;

    // Check if recipient is in allowlist
    let allowlist = ALLOWLIST.may_load(deps.storage, &recipient)?;
    match allowlist {
        Some(_) => Ok(Response::default()),
        None => {
            // Check if recipient has voting power in the DAO
            let voting_power: VotingPowerAtHeightResponse = deps.querier.query_wasm_smart(
                dao_voting_module,
                &VotingQueryMsg::VotingPowerAtHeight {
                    address: recipient.to_string(),
                    height: Some(env.block.height),
                },
            )?;

            // Throw error if recipient has no voting power
            if voting_power.power.is_zero() {
                return Err(ContractError::Unauthorized {});
            }

            Ok(Response::default())
        }
    }
}

pub fn execute_update_allowlist(
    deps: DepsMut,
    info: MessageInfo,
    add: Vec<String>,
    remove: Vec<String>,
) -> Result<Response, ContractError> {
    // Check if sender is the owner (normally the DAO)
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    // Add addresses to allowlist
    for address in add {
        ALLOWLIST.save(deps.storage, &deps.api.addr_validate(&address)?, &())?;
    }

    // Remove addresses from allowlist
    for address in remove {
        ALLOWLIST.remove(deps.storage, &deps.api.addr_validate(&address)?);
    }

    Ok(Response::default().add_attribute("action", "update_allowlist"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Dao {} => to_json_binary(&DAO.load(deps.storage)?),
        QueryMsg::DaoVotingPowerModule {} => to_json_binary(&DAO_VOTING_MODULE.load(deps.storage)?),
        QueryMsg::Allowlist { start_after, limit } => query_allowlist(deps, start_after, limit),
        QueryMsg::IsAllowed { address } => to_json_binary(&query_is_allowed(deps, address)?),
    }
}

pub fn query_allowlist(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Binary> {
    let addr: Addr;
    let start = match start_after {
        None => None,
        Some(addr_str) => {
            addr = deps.api.addr_validate(&addr_str)?;
            Some(Bound::exclusive(&addr))
        }
    };

    let limit = limit.unwrap_or(30).min(30) as usize;
    let allowlist = ALLOWLIST
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (k, _) = item?;
            Ok(k)
        })
        .collect::<StdResult<Vec<Addr>>>()?;

    to_json_binary(&allowlist)
}

pub fn query_is_allowed(deps: Deps, address: String) -> StdResult<bool> {
    let allowlist = ALLOWLIST.may_load(deps.storage, &deps.api.addr_validate(&address)?)?;
    match allowlist {
        Some(_) => Ok(true),
        None => {
            let dao_voting_module = DAO_VOTING_MODULE.load(deps.storage)?;

            // Check if recipient has voting power in the DAO
            let voting_power: VotingPowerAtHeightResponse = deps.querier.query_wasm_smart(
                dao_voting_module,
                &VotingQueryMsg::VotingPowerAtHeight {
                    address: address.to_string(),
                    height: None,
                },
            )?;

            // Throw error if recipient has no voting power
            if voting_power.power.is_zero() {
                return Ok(false);
            }

            Ok(true)
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    // Set contract to version to latest
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}
