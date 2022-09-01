#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, CosmosMsg, Deps, DepsMut, Empty, Env, MessageInfo, Order, Reply,
    Response, StdResult, Storage, SubMsg, Timestamp, Uint128, WasmMsg,
};
use cw2::set_contract_version;
use cw_core_interface::voting::IsActiveResponse;
use cw_storage_plus::{Bound, Item, Map};
use cw_utils::{parse_reply_instantiate_data, Duration, Expiration};
use indexable_hooks::Hooks;
use proposal_hooks::{new_proposal_hooks, proposal_status_changed_hooks};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use vote_hooks::new_vote_hooks;

use voting::deposit::CheckedDepositInfo;
use voting::pre_propose::{PreProposeInfo, ProposalCreationPolicy};
use voting::proposal::{DEFAULT_LIMIT, MAX_PROPOSAL_SIZE};
use voting::reply::{mask_proposal_execution_proposal_id, TaggedReplyId};
use voting::status::Status;
use voting::threshold::Threshold;
use voting::voting::{get_total_power, get_voting_power, validate_voting_period, Vote, Votes};

use crate::msg::MigrateMsg;
use crate::proposal::SingleChoiceProposal;
use crate::state::Config;
use crate::{
    error::ContractError,
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
    proposal::advance_proposal_id,
    query::ProposalListResponse,
    query::{ProposalResponse, VoteInfo, VoteListResponse, VoteResponse},
    state::{Ballot, BALLOTS, CONFIG, PROPOSALS, PROPOSAL_COUNT, PROPOSAL_HOOKS, VOTE_HOOKS},
};

pub(crate) const CONTRACT_NAME: &str = "crates.io:cw-1proposal-single";
pub(crate) const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    msg.threshold.validate()?;

    let dao = info.sender;

    let (min_voting_period, max_voting_period) =
        validate_voting_period(msg.min_voting_period, msg.max_voting_period)?;

    let (initial_policy, pre_propose_messages) = msg
        .pre_propose_info
        .into_initial_policy_and_messages(env.contract.address, deps.as_ref())?;

    let config = Config {
        threshold: msg.threshold,
        max_voting_period,
        min_voting_period,
        only_members_execute: msg.only_members_execute,
        dao: dao.clone(),
        allow_revoting: msg.allow_revoting,
        close_proposal_on_execution_failure: msg.close_proposal_on_execution_failure,
        proposal_creation_policy: initial_policy,
    };

    // Initialize proposal count to zero so that queries return zero
    // instead of None.
    PROPOSAL_COUNT.save(deps.storage, &0)?;
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::default()
        .add_submessages(pre_propose_messages)
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
            min_voting_period,
            only_members_execute,
            allow_revoting,
            dao,
            close_proposal_on_execution_failure,
            pre_propose_info,
        } => execute_update_config(
            deps,
            env,
            info,
            threshold,
            max_voting_period,
            min_voting_period,
            only_members_execute,
            allow_revoting,
            dao,
            close_proposal_on_execution_failure,
            pre_propose_info,
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

    // Check that the sender is permitted to create proposals.
    if !config.proposal_creation_policy.is_permitted(&sender) {
        return Err(ContractError::Unauthorized {});
    }

    // TODO(zeke): Should we move these checks into the pre-propose
    // module?
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

    let expiration = config.max_voting_period.after(&env.block);

    let total_power = get_total_power(deps.as_ref(), config.dao, Some(env.block.height))?;

    let proposal = {
        // Limit mutability to this block.
        let mut proposal = SingleChoiceProposal {
            title,
            description,
            proposer: sender.clone(),
            start_height: env.block.height,
            min_voting_period: config.min_voting_period.map(|min| min.after(&env.block)),
            expiration,
            threshold: config.threshold,
            total_power,
            msgs,
            status: Status::Open,
            votes: Votes::zero(),
            allow_revoting: config.allow_revoting,
            created: env.block.time,
            last_updated: env.block.time,
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

    let hooks = new_proposal_hooks(PROPOSAL_HOOKS, deps.storage, id, sender.clone())?;
    Ok(Response::default()
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
    // Update proposal's last updated timestamp.
    prop.last_updated = env.block.time;

    PROPOSALS.save(deps.storage, proposal_id, &prop)?;

    let response = {
        if !prop.msgs.is_empty() {
            let execute_message = WasmMsg::Execute {
                contract_addr: config.dao.to_string(),
                msg: to_binary(&cw_core::msg::ExecuteMsg::ExecuteProposalHook { msgs: prop.msgs })?,
                funds: vec![],
            };
            match config.close_proposal_on_execution_failure {
                true => {
                    let masked_proposal_id = mask_proposal_execution_proposal_id(proposal_id);
                    Response::default()
                        .add_submessage(SubMsg::reply_on_error(execute_message, masked_proposal_id))
                }
                false => Response::default().add_message(execute_message),
            }
        } else {
            Response::default()
        }
    };

    let hooks = proposal_status_changed_hooks(
        PROPOSAL_HOOKS,
        deps.storage,
        proposal_id,
        old_status.to_string(),
        prop.status.to_string(),
    )?;
    Ok(response
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
                        // Remove the old vote if this is a re-vote.
                        prop.votes
                            .remove_vote(current_ballot.vote, current_ballot.power);
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

    prop.status = Status::Closed;
    // Update proposal's last updated timestamp.
    prop.last_updated = env.block.time;
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
        .add_attribute("proposal_id", proposal_id.to_string()))
}

#[allow(clippy::too_many_arguments)]
pub fn execute_update_config(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    threshold: Threshold,
    max_voting_period: Duration,
    min_voting_period: Option<Duration>,
    only_members_execute: bool,
    allow_revoting: bool,
    dao: String,
    close_proposal_on_execution_failure: bool,
    pre_propose_info: PreProposeInfo,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // Only the DAO may call this method.
    if info.sender != config.dao {
        return Err(ContractError::Unauthorized {});
    }

    threshold.validate()?;
    let dao = deps.api.addr_validate(&dao)?;

    let (min_voting_period, max_voting_period) =
        validate_voting_period(min_voting_period, max_voting_period)?;
    let (initial_policy, pre_propose_messages) =
        pre_propose_info.into_initial_policy_and_messages(env.contract.address, deps.as_ref())?;

    CONFIG.save(
        deps.storage,
        &Config {
            threshold,
            max_voting_period,
            min_voting_period,
            only_members_execute,
            allow_revoting,
            dao,
            close_proposal_on_execution_failure,
            proposal_creation_policy: initial_policy,
        },
    )?;

    Ok(Response::default()
        .add_submessages(pre_propose_messages)
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
        QueryMsg::GetVote { proposal_id, voter } => query_vote(deps, proposal_id, voter),
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
        .collect::<Result<Vec<(u64, SingleChoiceProposal)>, _>>()?
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
        .collect::<Result<Vec<(u64, SingleChoiceProposal)>, _>>()?
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
pub fn migrate(deps: DepsMut, env: Env, msg: MigrateMsg) -> Result<Response, ContractError> {
    // Set contract to version to latest
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // This proposal version is from commit
    // e531c760a5d057329afd98d62567aaa4dca2c96f (v1.0.0) and code ID
    // 427.
    #[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
    struct V1Proposal {
        pub title: String,
        pub description: String,
        pub proposer: Addr,
        pub start_height: u64,
        pub min_voting_period: Option<Expiration>,
        pub expiration: Expiration,
        pub threshold: Threshold,
        pub total_power: Uint128,
        pub msgs: Vec<CosmosMsg<Empty>>,
        pub status: Status,
        pub votes: Votes,
        pub allow_revoting: bool,
        // FIXME(zeke): this type was changed and now a v1 proposal
        // will not deserialize into it.
        pub deposit_info: Option<CheckedDepositInfo>,
    }

    /// This config version is from commit
    /// e531c760a5d057329afd98d62567aaa4dca2c96f (v1.0.0).
    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
    pub struct V1Config {
        pub threshold: Threshold,
        pub max_voting_period: Duration,
        pub min_voting_period: Option<Duration>,
        pub only_members_execute: bool,
        pub allow_revoting: bool,
        pub dao: Addr,
        pub deposit_info: Option<CheckedDepositInfo>,
    }

    match msg {
        MigrateMsg::FromV1 {
            close_proposal_on_execution_failure,
        } => {
            // Update the stored config to have the new
            // `close_proposal_on_execution_falure` field.
            let config_item: Item<V1Config> = Item::new("config");
            let current_config = config_item.load(deps.storage)?;
            CONFIG.save(
                deps.storage,
                &Config {
                    threshold: current_config.threshold,
                    max_voting_period: current_config.max_voting_period,
                    min_voting_period: current_config.min_voting_period,
                    only_members_execute: current_config.only_members_execute,
                    allow_revoting: current_config.allow_revoting,
                    dao: current_config.dao,
                    // Loads of text, but we're only updating this field.
                    close_proposal_on_execution_failure,
                    // TODO(zeke): what we actually do here will
                    // depend on the specifics of how we instantiate
                    // deposit modules.
                    proposal_creation_policy: todo!(),
                },
            )?;

            // Update the module's proposals to v2.

            // Retrieve current map from storage
            let current_map: Map<u64, V1Proposal> = Map::new("proposals");
            let current = current_map
                .range(deps.storage, None, None, Order::Ascending)
                .collect::<StdResult<Vec<(u64, V1Proposal)>>>()?;

            // Add migrated entries to new map.
            // Based on gas usage testing, we estimate that we will be able to migrate ~4200
            // proposals at a time before reaching the block max_gas limit.
            current
                .into_iter()
                .try_for_each::<_, StdResult<()>>(|(id, prop)| {
                    let migrated_proposal = SingleChoiceProposal {
                        title: prop.title,
                        description: prop.description,
                        proposer: prop.proposer,
                        start_height: prop.start_height,
                        min_voting_period: prop.min_voting_period,
                        expiration: prop.expiration,
                        threshold: prop.threshold,
                        total_power: prop.total_power,
                        msgs: prop.msgs,
                        status: prop.status,
                        votes: prop.votes,
                        allow_revoting: prop.allow_revoting,
                        // CosmWasm does not expose a way to query the timestamp
                        // of a block given block height. As such, we assign migrated
                        // proposals a created timestamp of 0 so that the frontend may
                        // query for the true timestamp given `start_height`.
                        created: Timestamp::from_seconds(0),
                        last_updated: env.block.time,
                    };

                    PROPOSALS.save(deps.storage, id, &migrated_proposal)?;

                    Ok(())
                })?;

            todo!("update migration logic to create new pre-propose modules.")
        }

        MigrateMsg::FromCompatible {} => Ok(Response::default()),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    let repl = TaggedReplyId::new(msg.id)?;
    match repl {
        TaggedReplyId::FailedProposalExecution(proposal_id) => {
            PROPOSALS.update(deps.storage, proposal_id, |prop| match prop {
                Some(mut prop) => {
                    prop.status = Status::ExecutionFailed;
                    // Update proposal's last updated timestamp.
                    prop.last_updated = env.block.time;
                    Ok(prop)
                }
                None => Err(ContractError::NoSuchProposal { id: proposal_id }),
            })?;
            Ok(Response::new().add_attribute("proposal_execution_failed", proposal_id.to_string()))
        }
        TaggedReplyId::FailedProposalHook(idx) => {
            let config = CONFIG.load(deps.storage)?;
            let addr = PROPOSAL_HOOKS.remove_hook_by_index(deps.storage, idx)?;

            // If the address that failed to respond to the proposal
            // hook is the pre-proposal module we "fail open" by
            // resetting the proposal creation policy to anyone.
            if config.proposal_creation_policy.addr_is_my_module(&addr) {
                let mut config = config;
                config.proposal_creation_policy = ProposalCreationPolicy::Anyone {};
                CONFIG.save(deps.storage, &config)?;
            }

            Ok(Response::new().add_attribute("removed_proposal_hook", format!("{addr}:{idx}")))
        }
        TaggedReplyId::FailedVoteHook(idx) => {
            let addr = VOTE_HOOKS.remove_hook_by_index(deps.storage, idx)?;
            Ok(Response::new().add_attribute("removed_vote_hook", format!("{addr}:{idx}")))
        }
        TaggedReplyId::PreProposeModuleInstantiation => {
            let res = parse_reply_instantiate_data(msg)?;
            let module = deps.api.addr_validate(&res.contract_address)?;

            // TODO(zeke): should we validate that the current state
            // is `ProposalCreationPolicy::Anyone`? Depends on what we
            // decide the appropriate intermediate state should be..
            CONFIG.update(deps.storage, |mut config| -> StdResult<_> {
                config.proposal_creation_policy = ProposalCreationPolicy::Module {
                    addr: module.clone(),
                };
                Ok(config)
            })?;

            // Add the module as a receiver of proposal hooks. Doing
            // this gives us two things:
            //
            // 1. The module will be removed if it fails to handle a
            //    hook (want to fail open).
            // 2. The module will be informed when proposals change
            //    their status. This, for example, lets the module return
            //    deposits when proposals close.
            add_hook(PROPOSAL_HOOKS, deps.storage, module)?;

            Ok(Response::new().add_attribute("update_pre_propose_module", res.contract_address))
        }
    }
}
