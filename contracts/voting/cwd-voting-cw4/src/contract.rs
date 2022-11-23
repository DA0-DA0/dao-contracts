#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdError, StdResult,
    SubMsg, Uint128, WasmMsg,
};
use cw2::set_contract_version;
use cw_utils::parse_reply_instantiate_data;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use crate::state::{DAO, GROUP_CONTRACT, TOTAL_WEIGHT, USER_WEIGHTS};

pub(crate) const CONTRACT_NAME: &str = "crates.io:cwd-voting-cw4";
pub(crate) const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const INSTANTIATE_GROUP_REPLY_ID: u64 = 0;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    if msg.initial_members.is_empty() {
        return Err(ContractError::NoMembers {});
    }
    let original_len = msg.initial_members.len();
    let mut initial_members = msg.initial_members;
    initial_members.sort_by(|a, b| a.addr.cmp(&b.addr));
    initial_members.dedup();
    let new_len = initial_members.len();

    if original_len != new_len {
        return Err(ContractError::DuplicateMembers {});
    }

    let mut total_weight = Uint128::zero();
    for member in initial_members.iter() {
        let member_addr = deps.api.addr_validate(&member.addr)?;
        if member.weight > 0 {
            // This works because query_voting_power_at_height will return 0 on address missing
            // from storage, so no need to store anything.
            let weight = Uint128::from(member.weight);
            USER_WEIGHTS.save(deps.storage, &member_addr, &weight, env.block.height)?;
            total_weight += weight;
        }
    }

    if total_weight.is_zero() {
        return Err(ContractError::ZeroTotalWeight {});
    }
    TOTAL_WEIGHT.save(deps.storage, &total_weight, env.block.height)?;

    // We need to set ourself as the CW4 admin it is then transferred to the DAO in the reply
    let msg = WasmMsg::Instantiate {
        admin: Some(info.sender.to_string()),
        code_id: msg.cw4_group_code_id,
        msg: to_binary(&cw4_group::msg::InstantiateMsg {
            admin: Some(env.contract.address.to_string()),
            members: initial_members,
        })?,
        funds: vec![],
        label: env.contract.address.to_string(),
    };

    let msg = SubMsg::reply_on_success(msg, INSTANTIATE_GROUP_REPLY_ID);

    DAO.save(deps.storage, &info.sender)?;

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_submessage(msg))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::MemberChangedHook { diffs } => {
            execute_member_changed_hook(deps, env, info, diffs)
        }
    }
}

pub fn execute_member_changed_hook(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    diffs: Vec<cw4::MemberDiff>,
) -> Result<Response, ContractError> {
    let group_contract = GROUP_CONTRACT.load(deps.storage)?;
    if info.sender != group_contract {
        return Err(ContractError::Unauthorized {});
    }

    let total_weight = TOTAL_WEIGHT.load(deps.storage)?;
    // As difference can be negative we need to keep track of both
    // In seperate counters to apply at once and prevent underflow
    let mut positive_difference: Uint128 = Uint128::zero();
    let mut negative_difference: Uint128 = Uint128::zero();
    for diff in diffs {
        let user_address = deps.api.addr_validate(&diff.key)?;
        let weight = diff.new.unwrap_or_default();
        let old = diff.old.unwrap_or_default();
        // Do we need to add to positive difference or negative difference
        if weight > old {
            positive_difference += Uint128::from(weight - old);
        } else {
            negative_difference += Uint128::from(old - weight);
        }

        if weight != 0 {
            USER_WEIGHTS.save(
                deps.storage,
                &user_address,
                &Uint128::from(weight),
                env.block.height,
            )?;
        } else if weight == 0 && weight != old {
            // This works because query_voting_power_at_height will return 0 on address missing
            // from storage, so no need to store anything.
            //
            // Note that we also check for weight != old: If for some reason this hook is triggered
            // with weight 0 for old and new values, we don't need to do anything.
            USER_WEIGHTS.remove(deps.storage, &user_address, env.block.height)?;
        }
    }
    let new_total_weight = total_weight
        .checked_add(positive_difference)
        .map_err(StdError::overflow)?
        .checked_sub(negative_difference)
        .map_err(StdError::overflow)?;
    TOTAL_WEIGHT.save(deps.storage, &new_total_weight, env.block.height)?;

    Ok(Response::new()
        .add_attribute("action", "member_changed_hook")
        .add_attribute("total_weight", new_total_weight.to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::VotingPowerAtHeight { address, height } => {
            query_voting_power_at_height(deps, env, address, height)
        }
        QueryMsg::TotalPowerAtHeight { height } => query_total_power_at_height(deps, env, height),
        QueryMsg::Info {} => query_info(deps),
        QueryMsg::GroupContract {} => to_binary(&GROUP_CONTRACT.load(deps.storage)?),
        QueryMsg::Dao {} => to_binary(&DAO.load(deps.storage)?),
    }
}

pub fn query_voting_power_at_height(
    deps: Deps,
    env: Env,
    address: String,
    height: Option<u64>,
) -> StdResult<Binary> {
    let address = deps.api.addr_validate(&address)?;
    let height = height.unwrap_or(env.block.height);
    let power = USER_WEIGHTS
        .may_load_at_height(deps.storage, &address, height)?
        .unwrap_or_default();

    to_binary(&cwd_interface::voting::VotingPowerAtHeightResponse { power, height })
}

pub fn query_total_power_at_height(deps: Deps, env: Env, height: Option<u64>) -> StdResult<Binary> {
    let height = height.unwrap_or(env.block.height);
    let power = TOTAL_WEIGHT
        .may_load_at_height(deps.storage, height)?
        .unwrap_or_default();
    to_binary(&cwd_interface::voting::TotalPowerAtHeightResponse { power, height })
}

pub fn query_info(deps: Deps) -> StdResult<Binary> {
    let info = cw2::get_contract_version(deps.storage)?;
    to_binary(&cwd_interface::voting::InfoResponse { info })
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    // Set contract to version to latest
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        INSTANTIATE_GROUP_REPLY_ID => {
            let res = parse_reply_instantiate_data(msg);
            match res {
                Ok(res) => {
                    let group_contract = GROUP_CONTRACT.may_load(deps.storage)?;
                    if group_contract.is_some() {
                        return Err(ContractError::DuplicateGroupContract {});
                    }
                    let group_contract = deps.api.addr_validate(&res.contract_address)?;
                    let dao = DAO.load(deps.storage)?;
                    GROUP_CONTRACT.save(deps.storage, &group_contract)?;
                    let msg1 = WasmMsg::Execute {
                        contract_addr: group_contract.to_string(),
                        msg: to_binary(&cw4_group::msg::ExecuteMsg::AddHook {
                            addr: env.contract.address.to_string(),
                        })?,
                        funds: vec![],
                    };
                    // Transfer admin status to the DAO
                    let msg2 = WasmMsg::Execute {
                        contract_addr: group_contract.to_string(),
                        msg: to_binary(&cw4_group::msg::ExecuteMsg::UpdateAdmin {
                            admin: Some(dao.to_string()),
                        })?,
                        funds: vec![],
                    };
                    Ok(Response::default()
                        .add_attribute("group_contract_address", group_contract)
                        .add_message(msg1)
                        .add_message(msg2))
                }
                Err(_) => Err(ContractError::GroupContractInstantiateError {}),
            }
        }
        _ => Err(ContractError::UnknownReplyId { id: msg.id }),
    }
}
