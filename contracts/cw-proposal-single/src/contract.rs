#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, CosmosMsg, Deps, DepsMut, Empty, Env, MessageInfo, Reply, Response,
    StdResult, Storage, WasmMsg,
};
use cw2::set_contract_version;
use cw_core_interface::voting::IsActiveResponse;
use cw_storage_plus::Bound;
use cw_utils::Duration;
use indexable_hooks::Hooks;
use proposal_hooks::{new_proposal_hooks, proposal_status_changed_hooks};
use vote_hooks::new_vote_hooks;

use voting::{Status, Threshold, Vote, Votes};

use crate::{
    error::ContractError,
    msg::{DepositInfo, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg},
    proposal::{advance_proposal_id, Proposal},
    query::ProposalListResponse,
    query::{ProposalResponse, VoteInfo, VoteListResponse, VoteResponse},
    state::{
        get_deposit_msg, get_return_deposit_msg, Ballot, Config, BALLOTS, CONFIG, PROPOSALS,
        PROPOSAL_COUNT, PROPOSAL_HOOKS, VOTE_HOOKS,
    },
    utils::{get_total_power, get_voting_power},
};

const CONTRACT_NAME: &str = "crates.io:cw-govmod-single";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Default limit for proposal pagination.
const DEFAULT_LIMIT: u64 = 30;
const MAX_PROPOSAL_SIZE: u64 = 30_000;

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
    let deposit_info = msg
        .deposit_info
        .map(|info| info.into_checked(deps.as_ref(), dao.clone()))
        .transpose()?;

    let config = Config {
        threshold: msg.threshold,
        max_voting_period: msg.max_voting_period,
        only_members_execute: msg.only_members_execute,
        dao: dao.clone(),
        deposit_info,
        allow_revoting: msg.allow_revoting,
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
        } => execute_propose(deps, env, info.sender, title, description, msgs),
        ExecuteMsg::Vote { proposal_id, vote } => execute_vote(deps, env, info, proposal_id, vote),
        ExecuteMsg::Execute { proposal_id } => execute_execute(deps, env, info, proposal_id),
        ExecuteMsg::Close { proposal_id } => execute_close(deps, env, info, proposal_id),
        ExecuteMsg::UpdateConfig {
            threshold,
            max_voting_period,
            only_members_execute,
            allow_revoting,
            dao,
            deposit_info,
        } => execute_update_config(
            deps,
            info,
            threshold,
            max_voting_period,
            only_members_execute,
            allow_revoting,
            dao,
            deposit_info,
        ),
        ExecuteMsg::AddProposalHook { address } => {
            execute_add_proposal_hook(deps, env, info, address)
        }
        ExecuteMsg::RemoveProposalHook { address } => {
            execute_remove_proposal_hook(deps, env, info, address)
        }
        ExecuteMsg::AddVoteHook { address } => execute_add_vote_hook(deps, env, info, address),
        ExecuteMsg::RemoveVoteHook { address } => {
            execute_remove_vote_hook(deps, env, info, address)
        }
    }
}

pub fn execute_propose(
    deps: DepsMut,
    env: Env,
    sender: Addr,
    title: String,
    description: String,
    msgs: Vec<CosmosMsg<Empty>>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    let voting_module: Addr = deps
        .querier
        .query_wasm_smart(config.dao.clone(), &cw_core::msg::QueryMsg::VotingModule {})?;

    // Voting modules are not required to implement this
    // query. Lacking an implementation they are active by default.
    let active_resp: IsActiveResponse = deps
        .querier
        .query_wasm_smart(
            voting_module,
            &cw_core_interface::voting::Query::IsActive {},
        )
        .unwrap_or(IsActiveResponse { active: true });

    if !active_resp.active {
        return Err(ContractError::InactiveDao {});
    }

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

    let expiration = config.max_voting_period.after(&env.block);

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
            allow_revoting: config.allow_revoting,
            deposit_info: config.deposit_info.clone(),
        };
        // Update the proposal's status. Addresses case where proposal
        // expires on the same block as it is created.
        proposal.update_status(&env.block);
        proposal
    };
    let id = advance_proposal_id(deps.storage)?;

    // Limit the size of proposals.
    //
    // The Juno mainnet has a larger limit for data that can be
    // uploaded as part of an execute message than it does for data
    // that can be queried as part of a query. This means that without
    // this check it is possible to create a proposal that can not be
    // queried.
    //
    // The size selected was determined by uploading versions of this
    // contract to the Juno mainnet until queries worked within a
    // reasonable margin of error.
    //
    // `to_vec` is the method used by cosmwasm to convert a struct
    // into it's byte representation in storage.
    let proposal_size = cosmwasm_std::to_vec(&proposal)?.len() as u64;
    if proposal_size > MAX_PROPOSAL_SIZE {
        return Err(ContractError::ProposalTooLarge {
            size: proposal_size,
            max: MAX_PROPOSAL_SIZE,
        });
    }

    PROPOSALS.save(deps.storage, id, &proposal)?;

    let deposit_msg = get_deposit_msg(&config.deposit_info, &env.contract.address, &sender)?;
    let hooks = new_proposal_hooks(PROPOSAL_HOOKS, deps.storage, id)?;
    Ok(Response::default()
        .add_messages(deposit_msg)
        .add_submessages(hooks)
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
        let power = get_voting_power(deps.as_ref(), info.sender.clone(), config.dao.clone(), None)?;
        if power.is_zero() {
            return Err(ContractError::Unauthorized {});
        }
    }

    let mut prop = PROPOSALS
        .may_load(deps.storage, proposal_id)?
        .ok_or(ContractError::NoSuchProposal { id: proposal_id })?;

    // Check here that the proposal is passed. Allow it to be executed
    // even if it is expired so long as it passed during its voting
    // period.
    let old_status = prop.status;
    prop.update_status(&env.block);
    if prop.status != Status::Passed {
        return Err(ContractError::NotPassed {});
    }

    prop.status = Status::Executed;
    PROPOSALS.save(deps.storage, proposal_id, &prop)?;

    let refund_message = match prop.deposit_info {
        Some(deposit_info) => get_return_deposit_msg(&deposit_info, &prop.proposer)?,
        None => vec![],
    };

    let response = if !prop.msgs.is_empty() {
        let execute_message = WasmMsg::Execute {
            contract_addr: config.dao.to_string(),
            msg: to_binary(&cw_core::msg::ExecuteMsg::ExecuteProposalHook { msgs: prop.msgs })?,
            funds: vec![],
        };
        Response::<Empty>::default().add_message(execute_message)
    } else {
        Response::default()
    };

    let hooks = proposal_status_changed_hooks(
        PROPOSAL_HOOKS,
        deps.storage,
        proposal_id,
        old_status.to_string(),
        prop.status.to_string(),
    )?;
    Ok(response
        .add_messages(refund_message)
        .add_submessages(hooks)
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

    let mut previous_ballot = None;
    BALLOTS.update(
        deps.storage,
        (proposal_id, info.sender.clone()),
        |bal| match bal {
            Some(current_ballot) => {
                if prop.allow_revoting {
                    if current_ballot.vote == vote {
                        // Don't allow casting the same vote more than
                        // once. This seems liable to be confusing
                        // behavior.
                        Err(ContractError::AlreadyCast {})
                    } else {
                        previous_ballot = Some(current_ballot);
                        Ok(Ballot {
                            power: vote_power,
                            vote,
                        })
                    }
                } else {
                    Err(ContractError::AlreadyVoted {})
                }
            }
            None => Ok(Ballot {
                power: vote_power,
                vote,
            }),
        },
    )?;

    let old_status = prop.status;

    // Remove the old vote if this is a re-vote.
    if let Some(ballot) = previous_ballot {
        prop.votes.remove_vote(ballot.vote, ballot.power)
    }

    prop.votes.add_vote(vote, vote_power);
    prop.update_status(&env.block);

    PROPOSALS.save(deps.storage, proposal_id, &prop)?;

    let new_status = prop.status;
    let change_hooks = proposal_status_changed_hooks(
        PROPOSAL_HOOKS,
        deps.storage,
        proposal_id,
        old_status.to_string(),
        new_status.to_string(),
    )?;
    let vote_hooks = new_vote_hooks(
        VOTE_HOOKS,
        deps.storage,
        proposal_id,
        info.sender.to_string(),
        vote.to_string(),
    )?;
    Ok(Response::default()
        .add_submessages(change_hooks)
        .add_submessages(vote_hooks)
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

    // Update status to ensure that proposals which were open and have
    // expired are moved to "rejected."
    prop.update_status(&env.block);
    if prop.status != Status::Rejected {
        return Err(ContractError::WrongCloseStatus {});
    }

    let old_status = prop.status;

    let refund_message = match &prop.deposit_info {
        Some(deposit_info) => {
            if deposit_info.refund_failed_proposals {
                get_return_deposit_msg(deposit_info, &prop.proposer)?
            } else {
                vec![]
            }
        }
        None => vec![],
    };

    prop.status = Status::Closed;
    PROPOSALS.save(deps.storage, proposal_id, &prop)?;

    let changed_hooks = proposal_status_changed_hooks(
        PROPOSAL_HOOKS,
        deps.storage,
        proposal_id,
        old_status.to_string(),
        prop.status.to_string(),
    )?;

    Ok(Response::default()
        .add_submessages(changed_hooks)
        .add_attribute("action", "close")
        .add_attribute("sender", info.sender)
        .add_messages(refund_message)
        .add_attribute("proposal_id", proposal_id.to_string()))
}

#[allow(clippy::too_many_arguments)]
pub fn execute_update_config(
    deps: DepsMut,
    info: MessageInfo,
    threshold: Threshold,
    max_voting_period: Duration,
    only_members_execute: bool,
    allow_revoting: bool,
    dao: String,
    deposit_info: Option<DepositInfo>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // Only the DAO may call this method.
    if info.sender != config.dao {
        return Err(ContractError::Unauthorized {});
    }

    threshold.validate()?;
    let dao = deps.api.addr_validate(&dao)?;
    let deposit_info = deposit_info
        .map(|info| info.into_checked(deps.as_ref(), dao.clone()))
        .transpose()?;

    CONFIG.save(
        deps.storage,
        &Config {
            threshold,
            max_voting_period,
            only_members_execute,
            allow_revoting,
            dao,
            deposit_info,
        },
    )?;

    Ok(Response::default()
        .add_attribute("action", "update_config")
        .add_attribute("sender", info.sender))
}
pub fn add_hook(
    hooks: Hooks,
    storage: &mut dyn Storage,
    validated_address: Addr,
) -> Result<(), ContractError> {
    hooks
        .add_hook(storage, validated_address)
        .map_err(ContractError::HookError)?;
    Ok(())
}

pub fn remove_hook(
    hooks: Hooks,
    storage: &mut dyn Storage,
    validate_address: Addr,
) -> Result<(), ContractError> {
    hooks
        .remove_hook(storage, validate_address)
        .map_err(ContractError::HookError)?;
    Ok(())
}

pub fn execute_add_proposal_hook(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    address: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if config.dao != info.sender {
        // Only DAO can add hooks
        return Err(ContractError::Unauthorized {});
    }

    let validated_address = deps.api.addr_validate(&address)?;

    add_hook(PROPOSAL_HOOKS, deps.storage, validated_address)?;

    Ok(Response::default()
        .add_attribute("action", "add_proposal_hook")
        .add_attribute("address", address))
}

pub fn execute_remove_proposal_hook(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    address: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if config.dao != info.sender {
        // Only DAO can remove hooks
        return Err(ContractError::Unauthorized {});
    }

    let validated_address = deps.api.addr_validate(&address)?;

    remove_hook(PROPOSAL_HOOKS, deps.storage, validated_address)?;

    Ok(Response::default()
        .add_attribute("action", "remove_proposal_hook")
        .add_attribute("address", address))
}

pub fn execute_add_vote_hook(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    address: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if config.dao != info.sender {
        // Only DAO can add hooks
        return Err(ContractError::Unauthorized {});
    }

    let validated_address = deps.api.addr_validate(&address)?;

    add_hook(VOTE_HOOKS, deps.storage, validated_address)?;

    Ok(Response::default()
        .add_attribute("action", "add_vote_hook")
        .add_attribute("address", address))
}

pub fn execute_remove_vote_hook(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    address: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if config.dao != info.sender {
        // Only DAO can remove hooks
        return Err(ContractError::Unauthorized {});
    }

    let validated_address = deps.api.addr_validate(&address)?;

    remove_hook(VOTE_HOOKS, deps.storage, validated_address)?;

    Ok(Response::default()
        .add_attribute("action", "remove_vote_hook")
        .add_attribute("address", address))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => query_config(deps),
        QueryMsg::Proposal { proposal_id } => query_proposal(deps, env, proposal_id),
        QueryMsg::ListProposals { start_after, limit } => {
            query_list_proposals(deps, env, start_after, limit)
        }
        QueryMsg::ProposalCount {} => query_proposal_count(deps),
        QueryMsg::Vote { proposal_id, voter } => query_vote(deps, proposal_id, voter),
        QueryMsg::ListVotes {
            proposal_id,
            start_after,
            limit,
        } => query_list_votes(deps, proposal_id, start_after, limit),
        QueryMsg::Info {} => query_info(deps),
        QueryMsg::ReverseProposals {
            start_before,
            limit,
        } => query_reverse_proposals(deps, env, start_before, limit),
        QueryMsg::ProposalHooks {} => to_binary(&PROPOSAL_HOOKS.query_hooks(deps)?),
        QueryMsg::VoteHooks {} => to_binary(&VOTE_HOOKS.query_hooks(deps)?),
    }
}

pub fn query_config(deps: Deps) -> StdResult<Binary> {
    let config = CONFIG.load(deps.storage)?;
    to_binary(&config)
}

pub fn query_proposal(deps: Deps, env: Env, id: u64) -> StdResult<Binary> {
    let proposal = PROPOSALS.load(deps.storage, id)?;
    to_binary(&proposal.into_response(&env.block, id))
}

pub fn query_list_proposals(
    deps: Deps,
    env: Env,
    start_after: Option<u64>,
    limit: Option<u64>,
) -> StdResult<Binary> {
    let min = start_after.map(Bound::exclusive);
    let limit = limit.unwrap_or(DEFAULT_LIMIT);
    let props: Vec<ProposalResponse> = PROPOSALS
        .range(deps.storage, min, None, cosmwasm_std::Order::Ascending)
        .take(limit as usize)
        .collect::<Result<Vec<(u64, Proposal)>, _>>()?
        .into_iter()
        .map(|(id, proposal)| proposal.into_response(&env.block, id))
        .collect();

    to_binary(&ProposalListResponse { proposals: props })
}

pub fn query_reverse_proposals(
    deps: Deps,
    env: Env,
    start_before: Option<u64>,
    limit: Option<u64>,
) -> StdResult<Binary> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT);
    let max = start_before.map(Bound::exclusive);
    let props: Vec<ProposalResponse> = PROPOSALS
        .range(deps.storage, None, max, cosmwasm_std::Order::Descending)
        .take(limit as usize)
        .collect::<Result<Vec<(u64, Proposal)>, _>>()?
        .into_iter()
        .map(|(id, proposal)| proposal.into_response(&env.block, id))
        .collect();

    to_binary(&ProposalListResponse { proposals: props })
}

pub fn query_proposal_count(deps: Deps) -> StdResult<Binary> {
    let proposal_count = PROPOSAL_COUNT.load(deps.storage)?;
    to_binary(&proposal_count)
}

pub fn query_vote(deps: Deps, proposal_id: u64, voter: String) -> StdResult<Binary> {
    let voter = deps.api.addr_validate(&voter)?;
    let ballot = BALLOTS.may_load(deps.storage, (proposal_id, voter.clone()))?;
    let vote = ballot.map(|ballot| VoteInfo {
        voter,
        vote: ballot.vote,
        power: ballot.power,
    });
    to_binary(&VoteResponse { vote })
}

pub fn query_list_votes(
    deps: Deps,
    proposal_id: u64,
    start_after: Option<String>,
    limit: Option<u64>,
) -> StdResult<Binary> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT);
    let start_after = start_after
        .map(|addr| deps.api.addr_validate(&addr))
        .transpose()?;
    let min = start_after.map(Bound::<Addr>::exclusive);

    let votes = BALLOTS
        .prefix(proposal_id)
        .range(deps.storage, min, None, cosmwasm_std::Order::Ascending)
        .take(limit as usize)
        .map(|item| {
            let (voter, ballot) = item?;
            Ok(VoteInfo {
                voter,
                vote: ballot.vote,
                power: ballot.power,
            })
        })
        .collect::<StdResult<Vec<_>>>()?;

    to_binary(&VoteListResponse { votes })
}

pub fn query_info(deps: Deps) -> StdResult<Binary> {
    let info = cw2::get_contract_version(deps.storage)?;
    to_binary(&cw_core_interface::voting::InfoResponse { info })
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    // Don't do any state migrations.
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    if msg.id % 2 == 0 {
        // Proposal hook so we can just divide by two for index
        let idx = msg.id / 2;
        PROPOSAL_HOOKS.remove_hook_by_index(deps.storage, idx)?;
        Ok(Response::new())
    } else {
        // Vote hook so we can minus one then divid by two for index
        let idx = (msg.id - 1) / 2;
        VOTE_HOOKS.remove_hook_by_index(deps.storage, idx)?;
        Ok(Response::new())
    }
}
