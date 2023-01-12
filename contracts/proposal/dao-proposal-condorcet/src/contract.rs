#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};

use cw2::set_contract_version;
use dao_voting::status::Status;
use dao_voting::threshold::validate_quorum;
use dao_voting::voting::{get_total_power, get_voting_power, validate_voting_period};

use crate::config::Config;
use crate::error::ContractError;
use crate::msg::{Choice, ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::proposal::Proposal;
use crate::state::{next_proposal_id, CONFIG, DAO, PROPOSALS, TALLYS};
use crate::tally::{Tally, Winner};
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
            voting_period,
            min_voting_period,
        },
    )?;

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
        ExecuteMsg::Propose { choices } => execute_propose(deps, env, info, choices),
        ExecuteMsg::Vote { proposal_id, vote } => execute_vote(deps, info, proposal_id, vote),
        ExecuteMsg::Execute { proposal_id } => execute_execute(deps, env, info, proposal_id),
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
    let expiration = config.voting_period.after(&env.block);
    let total_power = get_total_power(deps.as_ref(), dao.clone(), None)?;

    let tally = Tally::new(choices.len().try_into().unwrap(), total_power);

    let mut proposal = Proposal::new(
        id,
        choices,
        config.quorum,
        expiration,
        env.block.height,
        total_power,
    );
    proposal.update_status(&env.block, &tally);

    PROPOSALS.save(deps.storage, id, &proposal)?;
    TALLYS.save(deps.storage, id, &tally)?;

    Ok(Response::default().add_attribute("method", "propose"))
}

fn execute_vote(
    deps: DepsMut,
    info: MessageInfo,
    proposal_id: u32,
    vote: Vec<u32>,
) -> Result<Response, ContractError> {
    let proposal = PROPOSALS.load(deps.storage, proposal_id)?;
    let sender_power = get_voting_power(
        deps.as_ref(),
        info.sender,
        DAO.load(deps.storage)?,
        Some(proposal.start_height),
    )?;
    if sender_power.is_zero() {
        return Err(ContractError::ZeroVotingPower {});
    }

    let mut tally = TALLYS.load(deps.storage, proposal_id)?;
    let vote = Vote::new(vote, tally.candidates())?;
    tally.add_vote(vote, sender_power);
    TALLYS.save(deps.storage, proposal_id, &tally)?;

    Ok(Response::default().add_attribute("method", "vote"))
}

fn execute_execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    proposal_id: u32,
) -> Result<Response, ContractError> {
    let mut proposal = PROPOSALS.load(deps.storage, proposal_id)?;
    let sender_power = get_voting_power(
        deps.as_ref(),
        info.sender,
        DAO.load(deps.storage)?,
        Some(proposal.start_height),
    )?;
    if sender_power.is_zero() {
        return Err(ContractError::ZeroVotingPower {});
    }

    let tally = TALLYS.load(deps.storage, proposal_id)?;
    if proposal.status(&env.block, &tally) != Status::Passed {
        return Err(ContractError::Unexecutable {});
    }
    let winner = match tally.winner {
        Winner::Some(i) | Winner::Undisputed(i) => i,
        _ => return Err(ContractError::Unexecutable {}),
    };

    proposal.set_executed();
    PROPOSALS.save(deps.storage, proposal_id, &proposal)?;

    Ok(Response::default()
        .add_attribute("method", "execute")
        .add_messages(proposal.choices.swap_remove(winner as usize).msgs))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {}
}
