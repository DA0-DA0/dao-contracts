#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdResult};

use cw2::set_contract_version;
use dao_voting::reply::TaggedReplyId;
use dao_voting::threshold::validate_quorum;
use dao_voting::voting::{get_total_power, get_voting_power, validate_voting_period};

use crate::config::Config;
use crate::error::ContractError;
use crate::msg::{Choice, ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::proposal::{Proposal, Status};
use crate::state::{next_proposal_id, CONFIG, DAO, PROPOSALS, TALLYS};
use crate::tally::Tally;
use crate::vote::Vote;

pub(crate) const CONTRACT_NAME: &str = "crates.io:dao-proposal-condorcet";
pub(crate) const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    validate_quorum(&msg.quorum)?;
    let (min_voting_period, voting_period) =
        validate_voting_period(msg.min_voting_period, msg.voting_period)?;

    DAO.save(deps.storage, &info.sender)?;
    CONFIG.save(
        deps.storage,
        &Config {
            quorum: msg.quorum,
            close_proposals_on_execution_failure: msg.close_proposals_on_execution_failure,
            voting_period,
            min_voting_period,
        },
    )?;

    Ok(Response::default()
        .add_attribute("method", "instantiate")
        .add_attribute("creator", info.sender))
}

// the key to this contract being gas efficent [1] is that
//
// ```
// gas(vote) <= gas(propose) && gas(execute) <= gas(propose)
// ```
//
// that being true, you will never be able to create a proposal that
// can not be voted on and executed inside gas limits.
//
// in terms of storage costs:
//
// propose: proposal_load + proposal_store + tally_load + tally_store + config_load
// execute: proposal_load + proposal_store + tally_load
// vote:    tally_load + tally_store
//
// so we're good there.
//
// in terms of other costs:
//
// propose: query_voting_power + compute_winner [2]
// execute: query_voting_power
// vote:    query_voting_power + compute_winner
//
// so we're good there as well.
//
// [1] we need to be gas efficent in this way because the size of the
//     Tally type grows with candidates^2 and thus can be too large to
//     load from storage. we need to make sure that if this is the
//     case, the proposal fails to be created. the bad outcome we're
//     trying to avoid here is a proposal that is created but can not
//     be voted on or executed.
// [2] Tally::new computes the winner over the new matrix so that this
//     is the case.

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Propose { choices } => execute_propose(deps, env, info, choices),
        ExecuteMsg::Vote { proposal_id, vote } => execute_vote(deps, info, proposal_id, vote),
        ExecuteMsg::Execute { proposal_id } => execute_execute(deps, env, info, proposal_id),
        ExecuteMsg::Close { proposal_id } => execute_close(deps, env, proposal_id),
    }
}

fn execute_propose(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    choices: Vec<Choice>,
) -> Result<Response, ContractError> {
    let dao = DAO.load(deps.storage)?;
    let sender_voting_power = get_voting_power(deps.as_ref(), info.sender, dao.clone(), None)?;
    if sender_voting_power.is_zero() {
        return Err(ContractError::ZeroVotingPower {});
    }

    let config = CONFIG.load(deps.storage)?;

    let id = next_proposal_id(deps.storage)?;
    let total_power = get_total_power(deps.as_ref(), dao.clone(), None)?;

    let tally = Tally::new(choices.len(), total_power, env.block.height);
    TALLYS.save(deps.storage, id, &tally)?;

    let mut proposal = Proposal::new(&env.block, &config, id, choices, total_power);
    proposal.update_status(&env.block, &tally);
    PROPOSALS.save(deps.storage, id, &proposal)?;

    Ok(Response::default().add_attribute("method", "propose"))
}

fn execute_vote(
    deps: DepsMut,
    info: MessageInfo,
    proposal_id: u32,
    vote: Vec<u32>,
) -> Result<Response, ContractError> {
    let tally = TALLYS.load(deps.storage, proposal_id)?;
    let sender_power = get_voting_power(
        deps.as_ref(),
        info.sender,
        DAO.load(deps.storage)?,
        Some(tally.start_height),
    )?;
    if sender_power.is_zero() {
        Err(ContractError::ZeroVotingPower {})
    } else {
        let vote = Vote::new(vote, tally.candidates())?;

        let mut tally = tally;
        tally.add_vote(vote, sender_power);
        TALLYS.save(deps.storage, proposal_id, &tally)?;

        Ok(Response::default().add_attribute("method", "vote"))
    }
}

fn execute_execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    proposal_id: u32,
) -> Result<Response, ContractError> {
    let tally = TALLYS.load(deps.storage, proposal_id)?;
    let dao = DAO.load(deps.storage)?;
    let sender_power = get_voting_power(
        deps.as_ref(),
        info.sender,
        dao.clone(),
        Some(tally.start_height),
    )?;
    if sender_power.is_zero() {
        return Err(ContractError::ZeroVotingPower {});
    }

    let proposal = PROPOSALS.load(deps.storage, proposal_id)?;
    if let Status::Passed { winner } = proposal.status(&env.block, &tally) {
        let mut proposal = proposal;
        let msgs = proposal.set_executed(dao, winner)?;
        PROPOSALS.save(deps.storage, proposal_id, &proposal)?;

        Ok(Response::default()
            .add_attribute("method", "execute")
            .add_submessage(msgs))
    } else {
        Err(ContractError::Unexecutable {})
    }
}

fn execute_close(deps: DepsMut, env: Env, proposal_id: u32) -> Result<Response, ContractError> {
    let tally = TALLYS.load(deps.storage, proposal_id)?;
    let proposal = PROPOSALS.load(deps.storage, proposal_id)?;
    if let Status::Rejected = proposal.status(&env.block, &tally) {
        let mut proposal = proposal;
        proposal.set_closed();
        PROPOSALS.save(deps.storage, proposal_id, &proposal)?;

        Ok(Response::default().add_attribute("method", "close"))
    } else {
        Err(ContractError::Unclosable {})
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {}
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    let repl = TaggedReplyId::new(msg.id)?;
    match repl {
        TaggedReplyId::FailedProposalExecution(proposal_id) => {
            let mut proposal = PROPOSALS.load(deps.storage, proposal_id as u32)?;
            proposal.set_execution_failed();
            PROPOSALS.save(deps.storage, proposal_id as u32, &proposal)?;
            Ok(Response::default()
                .add_attribute("proposal_execution_failed", proposal_id.to_string()))
        }
        _ => unimplemented!("pre-propose and hooks not yet supported"),
    }
}
