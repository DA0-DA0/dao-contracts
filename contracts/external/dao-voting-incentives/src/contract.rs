#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{BankMsg, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw2::set_contract_version;
use dao_hooks::vote::VoteHookMsg;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use crate::state::{DAO, VOTING_INCENTIVES};

pub(crate) const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
pub(crate) const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

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

    // Save voting incentives config

    // TODO Check initial deposit is enough to pay out rewards for at
    // least one epoch

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
        ExecuteMsg::Claim {} => execute_claim(deps, env, info),
        ExecuteMsg::VoteHook(msg) => execute_vote_hook(deps, env, info, msg),
    }
}

// TODO how to claim for many epochs efficiently?
pub fn execute_claim(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    // Check epoch should advance

    // Save last claimed epoch

    // Load user vote count for epoch?

    // Load prop count for epoch

    // Load voting incentives config
    let voting_incentives = VOTING_INCENTIVES.load(deps.storage)?;

    // Need total vote count for epoch
    // Rewards = (user vote count / prop count) / total_vote_count * voting incentives

    // Pay out rewards

    Ok(Response::default().add_attribute("action", "claim"))
}

// TODO support cw20 tokens
// TODO make sure config can't lock DAO
pub fn execute_vote_hook(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: VoteHookMsg,
) -> Result<Response, ContractError> {
    // Check epoch should advance

    // TODO need some state to handle this
    // Check that the vote is not a changed vote (i.e. the user has already voted
    // on the prop).

    // Save (user, epoch), vote count
    // Update (epoch, prop count)

    Ok(Response::default().add_attribute("action", "vote_hook"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Rewards { address } => unimplemented!(),
        QueryMsg::Config {} => unimplemented!(),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    // Set contract to version to latest
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}
