#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    Addr, Binary, CosmosMsg, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdResult,
};
use cw2::set_contract_version;
use cw_utils::{Duration, Expiration};

use crate::{
    error::ContractError,
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
    proposal::{Proposal, Status, Votes},
    state::{Config, CONFIG},
    utils::{get_total_power, get_voting_power},
};

const CONTRACT_NAME: &str = "crates.io:cw-govmod-single";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    msg.threshold.validate()?;

    let dao = info.sender;

    let config = Config {
        threshold: msg.threshold,
        max_voting_period: msg.max_voting_period,
        only_members_execute: msg.only_members_execute,
        dao: dao.clone(),
    };

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::default()
        .add_attribute("action", "instantiate")
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
        ExecuteMsg::Propose {
            title,
            description,
            msgs,
            latest,
        } => execute_propose(deps, env, info.sender, title, description, msgs, latest),
        ExecuteMsg::Vote { proposal_id, vote } => todo!(),
        ExecuteMsg::Execute { proposal_id } => todo!(),
        ExecuteMsg::Close { proposal_id } => todo!(),
        ExecuteMsg::UpdateConfig {
            threshold,
            max_voting_period,
            only_members_execute,
        } => todo!(),
    }
}

pub fn execute_propose(
    deps: DepsMut,
    env: Env,
    sender: Addr,
    title: String,
    description: String,
    msgs: Vec<CosmosMsg<Empty>>,
    latest: Option<Expiration>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // Check that the sender is a member of the governance contract.
    let sender_power = get_voting_power(
        deps.as_ref(),
        sender.clone(),
        config.dao.clone(),
        Some(env.block.height),
    )?;
    if sender_power.is_zero() {
        return Err(ContractError::Unauthorized {});
    }

    // Set the expiration to the minimum of the proposal's `latest`
    // argument and the configured max voting period.
    let max_voting_expiration = config.max_voting_period.after(&env.block);
    let expiration = if let Some(latest) = latest {
        if latest <= max_voting_expiration {
            latest
        } else {
            return Err(ContractError::InvalidExpiration {});
        }
    } else {
        max_voting_expiration
    };

    let total_power = get_total_power(deps.as_ref(), config.dao, Some(env.block.height))?;

    let proposal = Proposal {
        title,
        description,
        proposer: sender,
        start_height: env.block.height,
        expiration,
        threshold: config.threshold,
        total_power,
        msgs,
        status: Status::Open,
        votes: Votes::zero(),
    };

    todo!()
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => todo!(),
        QueryMsg::Proposal { proposal_id } => todo!(),
        QueryMsg::ListProposals { start_after, limit } => todo!(),
        QueryMsg::ProposalCount {} => todo!(),
        QueryMsg::Vote { proposal_id, voter } => todo!(),
        QueryMsg::ListVotes {
            proposal_id,
            start_after,
            limit,
        } => todo!(),
        QueryMsg::Tally { proposal_id } => todo!(),
        QueryMsg::Info {} => todo!(),
    }
}
