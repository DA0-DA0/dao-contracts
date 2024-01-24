#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, BankMsg, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdResult,
    SubMsg,
};
use cw2::set_contract_version;
use dao_hooks::proposal::ProposalHookMsg;
use dao_voting::status::Status;

use crate::error::ContractError;
use crate::msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use crate::state::{DAO, PROPOSAL_INCENTIVES};

pub(crate) const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
pub(crate) const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const REPLY_PROPOSAL_HOOK_ID: u64 = 1;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Save DAO, assumes the sender is the DAO
    DAO.save(deps.storage, &deps.api.addr_validate(&msg.dao)?)?;

    // Save proposal incentives config
    PROPOSAL_INCENTIVES.save(deps.storage, &msg.proposal_incentives)?;

    // TODO Check initial deposit contains enough funds to pay out rewards
    // for at least one proposal

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
        ExecuteMsg::ProposalHook(msg) => execute_proposal_hook(deps, env, info, msg),
    }
}

// TODO support cw20 tokens
pub fn execute_proposal_hook(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ProposalHookMsg,
) -> Result<Response, ContractError> {
    let mut payout_msgs: Vec<SubMsg> = vec![];

    // Check prop status and type of hook
    match msg {
        ProposalHookMsg::ProposalStatusChanged { new_status, .. } => {
            // If prop status is success, add message to pay out rewards
            // Otherwise, do nothing
            if new_status == Status::Passed.to_string() {
                // Load proposal incentives config
                let proposal_incentives = PROPOSAL_INCENTIVES.load(deps.storage)?;

                // We handle payout messages in a SubMsg so the error be caught
                // if need be. This is to prevent running out of funds locking the DAO.
                payout_msgs.push(SubMsg::reply_on_error(
                    BankMsg::Send {
                        to_address: info.sender.to_string(),
                        amount: vec![proposal_incentives.rewards_per_proposal],
                    },
                    REPLY_PROPOSAL_HOOK_ID,
                ));
            }
        }
        _ => {}
    }

    Ok(Response::default()
        .add_attribute("action", "proposal_hook")
        .add_submessages(payout_msgs))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => query_config(deps),
    }
}

pub fn query_config(deps: Deps) -> StdResult<Binary> {
    let dao = DAO.load(deps.storage)?;
    let proposal_incentives = PROPOSAL_INCENTIVES.load(deps.storage)?;

    to_json_binary(&ConfigResponse {
        dao,
        proposal_incentives,
    })
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    // Set contract to version to latest
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(_deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        REPLY_PROPOSAL_HOOK_ID => {
            // If an error occurred with payout, we still return an ok response
            // because we don't want to fail the proposal hook and lock the DAO.
            Ok(Response::default())
        }
        _ => Err(ContractError::UnknownReplyID {}),
    }
}
