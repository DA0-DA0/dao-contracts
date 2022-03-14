#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, CosmosMsg, Deps, DepsMut, Empty, Env, MessageInfo, Response,
    StdResult, WasmMsg,
};
use cw2::set_contract_version;
use cw_utils::{Duration, Expiration};

use crate::{
    error::ContractError,
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
    proposal::{advance_proposal_id, Proposal, Status, Vote, Votes},
    state::{Ballot, Config, BALLOTS, CONFIG, PROPOSALS},
    threshold::Threshold,
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
        ExecuteMsg::Vote { proposal_id, vote } => execute_vote(deps, env, info, proposal_id, vote),
        ExecuteMsg::Execute { proposal_id } => execute_execute(deps, env, info, proposal_id),
        ExecuteMsg::Close { proposal_id } => execute_close(deps, env, info, proposal_id),
        ExecuteMsg::UpdateConfig {
            threshold,
            max_voting_period,
            only_members_execute,
            dao,
        } => execute_update_config(
            deps,
            info,
            threshold,
            max_voting_period,
            only_members_execute,
            dao,
        ),
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

    let proposal = {
        // Limit mutability to this block.
        let mut proposal = Proposal {
            title,
            description,
            proposer: sender.clone(),
            start_height: env.block.height,
            expiration,
            threshold: config.threshold,
            total_power,
            msgs,
            status: Status::Open,
            votes: Votes::zero(),
        };
        // Update the proposal's status. Addresses case where proposal
        // expires on the same block as it is created.
        proposal.update_status(&env.block);
        proposal
    };
    let id = advance_proposal_id(deps.storage)?;
    PROPOSALS.save(deps.storage, id, &proposal)?;

    Ok(Response::default()
        .add_attribute("action", "propose")
        .add_attribute("sender", sender)
        .add_attribute("proposal_id", id.to_string())
        .add_attribute("status", proposal.status.to_string()))
}

pub fn execute_execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    proposal_id: u64,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if config.only_members_execute {
        let power = get_voting_power(
            deps.as_ref(),
            info.sender.clone(),
            config.dao.clone(),
            Some(env.block.height),
        )?;
        if power.is_zero() {
            return Err(ContractError::Unauthorized {});
        }
    }

    let mut prop = PROPOSALS
        .may_load(deps.storage, proposal_id)?
        .ok_or(ContractError::NoSuchProposal { id: proposal_id })?;

    // Check here that the proposal is passed. Allow it to be
    // executed even if it is expired so long as it passed during its
    // voting period.
    if prop.is_passed(&env.block) {
        return Err(ContractError::NotPassed {});
    }
    prop.status = Status::Executed;
    PROPOSALS.save(deps.storage, proposal_id, &prop)?;

    let response = if !prop.msgs.is_empty() {
        let execute_message = WasmMsg::Execute {
            contract_addr: config.dao.to_string(),
            msg: to_binary(&cw_governance::msg::ExecuteMsg::ExecuteProposalHook {
                msgs: prop.msgs,
            })?,
            funds: vec![],
        };
        Response::<Empty>::default().add_message(execute_message)
    } else {
        Response::default()
    };

    Ok(response
        .add_attribute("action", "execute")
        .add_attribute("sender", info.sender)
        .add_attribute("proposal_id", proposal_id.to_string())
        .add_attribute("dao", config.dao))
}

pub fn execute_vote(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    proposal_id: u64,
    vote: Vote,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let mut prop = PROPOSALS
        .may_load(deps.storage, proposal_id)?
        .ok_or(ContractError::NoSuchProposal { id: proposal_id })?;
    if prop.current_status(&env.block) != Status::Open {
        return Err(ContractError::NotOpen { id: proposal_id });
    }

    let vote_power = get_voting_power(
        deps.as_ref(),
        info.sender.clone(),
        config.dao,
        Some(prop.start_height),
    )?;
    if vote_power.is_zero() {
        return Err(ContractError::NotRegistered {});
    }

    BALLOTS.update(
        deps.storage,
        (proposal_id, info.sender.clone()),
        |bal| match bal {
            Some(_) => Err(ContractError::AlreadyVoted {}),
            None => Ok(Ballot {
                power: vote_power,
                vote,
            }),
        },
    )?;

    prop.votes.add_vote(vote, vote_power);
    prop.update_status(&env.block);
    PROPOSALS.save(deps.storage, proposal_id, &prop)?;

    Ok(Response::default()
        .add_attribute("action", "vote")
        .add_attribute("sender", info.sender)
        .add_attribute("proposal_id", proposal_id.to_string())
        .add_attribute("position", vote.to_string())
        .add_attribute("status", prop.status.to_string()))
}

pub fn execute_close(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    proposal_id: u64,
) -> Result<Response, ContractError> {
    let mut prop = PROPOSALS.load(deps.storage, proposal_id)?;

    // Only open and expired or rejected proposals may be closed.
    match prop.status {
        Status::Rejected => (),
        Status::Open => {
            if !prop.expiration.is_expired(&env.block) {
                return Err(ContractError::NotExpired {});
            }
        }
        _ => return Err(ContractError::WrongCloseStatus {}),
    }

    prop.status = Status::Closed;
    PROPOSALS.save(deps.storage, proposal_id, &prop)?;

    Ok(Response::default()
        .add_attribute("action", "close")
        .add_attribute("sender", info.sender)
        .add_attribute("proposal_id", proposal_id.to_string()))
}

pub fn execute_update_config(
    deps: DepsMut,
    info: MessageInfo,
    threshold: Threshold,
    max_voting_period: Duration,
    only_members_execute: bool,
    dao: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    threshold.validate()?;
    let dao = deps.api.addr_validate(&dao)?;

    // Only the DAO may call this method.
    if info.sender != config.dao {
        return Err(ContractError::Unauthorized {});
    }

    CONFIG.save(
        deps.storage,
        &Config {
            threshold,
            max_voting_period,
            only_members_execute,
            dao,
        },
    )?;

    Ok(Response::default()
        .add_attribute("action", "update_config")
        .add_attribute("sender", info.sender))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => query_config(deps),
        QueryMsg::Proposal { proposal_id } => query_proposal(deps, proposal_id),
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

pub fn query_config(deps: Deps) -> StdResult<Binary> {
    let config = CONFIG.load(deps.storage)?;
    to_binary(&config)
}

pub fn query_proposal(deps: Deps, id: u64) -> StdResult<Binary> {
    let proposal = PROPOSALS.load(deps.storage, id)?;
    to_binary(&proposal)
}
