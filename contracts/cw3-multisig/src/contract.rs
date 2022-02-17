use std::cmp::Ordering;
use std::string::FromUtf8Error;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, CosmosMsg, Decimal, Deps, DepsMut, Empty, Env, MessageInfo, Order,
    Reply, Response, StdResult, SubMsg, Uint128, WasmMsg,
};

use cw2::set_contract_version;
use cw20::{BalanceResponse, Cw20CoinVerified, Cw20QueryMsg};
use cw3::{
    Status, Vote, VoteInfo, VoteListResponse, VoteResponse, VoterDetail, VoterListResponse,
    VoterResponse,
};
use cw4::{Cw4Contract, MemberChangedHookMsg, MemberDiff};
use cw4_group::msg::InstantiateMsg as Cw4InstantiateMsg;
use cw_storage_plus::Bound;
use cw_utils::{maybe_addr, parse_reply_instantiate_data, Expiration, ThresholdResponse};

use crate::error::ContractError;
use crate::helpers::{get_and_check_limit, map_proposal};
use crate::msg::{ExecuteMsg, GroupMsg, InstantiateMsg, QueryMsg};
use crate::query::{
    ConfigResponse, Cw20BalancesResponse, ProposalListResponse, ProposalResponse,
    TokenListResponse, VoteTallyResponse,
};
use crate::state::{
    next_id, Ballot, Config, Proposal, Votes, BALLOTS, CONFIG, GROUP_ADDRESS, PROPOSALS,
    TREASURY_TOKENS,
};

// Version info for migration info
const CONTRACT_NAME: &str = "crates.io:cw3-multisig";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

// Settings for pagination
const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;

// Reply IDs
const INSTANTIATE_CW4_GROUP_REPLY_ID: u64 = 0;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let cfg = Config {
        name: msg.name,
        description: msg.description,
        threshold: msg.threshold.clone(),
        max_voting_period: msg.max_voting_period,
        image_url: msg.image_url,
        only_members_execute: true,
    };
    CONFIG.save(deps.storage, &cfg)?;

    let mut msgs: Vec<SubMsg> = vec![];

    match msg.group {
        GroupMsg::InstantiateNewGroup {
            code_id,
            label,
            voters,
        } => {
            if voters.is_empty() {
                return Err(ContractError::NoVoters {});
            }

            // Instantiate group contract
            let msg = WasmMsg::Instantiate {
                code_id,
                funds: vec![],
                admin: Some(env.contract.address.to_string()),
                label,
                msg: to_binary(&Cw4InstantiateMsg {
                    admin: Some(env.contract.address.to_string()),
                    members: voters,
                })?,
            };

            // Throw error on instantiate message failing
            let msg = SubMsg::reply_on_success(msg, INSTANTIATE_CW4_GROUP_REPLY_ID);

            msgs.append(&mut vec![msg]);
        }

        GroupMsg::UseExistingGroup { addr } => {
            // Get group contract
            let group_addr = Cw4Contract(
                deps.api
                    .addr_validate(&addr)
                    .map_err(|_| ContractError::InvalidGroup { addr: addr.clone() })?,
            );

            // Validate threshold
            let total_weight = group_addr.total_weight(&deps.querier)?;
            msg.threshold.validate(total_weight)?;

            // Save group address
            GROUP_ADDRESS.save(deps.storage, &group_addr)?;
        }
    }

    Ok(Response::default().add_submessages(msgs))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response<Empty>, ContractError> {
    match msg {
        ExecuteMsg::Propose {
            title,
            description,
            msgs,
            latest,
        } => execute_propose(deps, env, info, title, description, msgs, latest),
        ExecuteMsg::Vote { proposal_id, vote } => execute_vote(deps, env, info, proposal_id, vote),
        ExecuteMsg::Execute { proposal_id } => execute_execute(deps, env, info, proposal_id),
        ExecuteMsg::Close { proposal_id } => execute_close(deps, env, info, proposal_id),
        ExecuteMsg::MemberChangedHook(MemberChangedHookMsg { diffs }) => {
            execute_membership_hook(deps, env, info, diffs)
        }
        ExecuteMsg::UpdateConfig(config) => execute_update_config(deps, env, info, config),
        ExecuteMsg::UpdateCw20TokenList { to_add, to_remove } => {
            execute_update_cw20_token_list(deps, env, info, to_add, to_remove)
        }
    }
}

pub fn execute_propose(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    title: String,
    description: String,
    msgs: Vec<CosmosMsg>,
    // we ignore earliest
    latest: Option<Expiration>,
) -> Result<Response<Empty>, ContractError> {
    // only members of the multisig can create a proposal
    let cfg = CONFIG.load(deps.storage)?;
    let group_addr = GROUP_ADDRESS.load(deps.storage)?;

    // Only members of the multisig can create a proposal
    // Non-voting members are special - they are allowed to create a proposal and
    // therefore "vote", but they aren't allowed to vote otherwise.
    // Such vote is also special, because despite having 0 weight it still counts when
    // counting threshold passing
    let vote_power = group_addr
        .is_member(&deps.querier, &info.sender, None)?
        .ok_or(ContractError::Unauthorized {})?;

    // max expires also used as default
    let max_expires = cfg.max_voting_period.after(&env.block);
    let mut expires = latest.unwrap_or(max_expires);
    let comp = expires.partial_cmp(&max_expires);
    if let Some(Ordering::Greater) = comp {
        expires = max_expires;
    } else if comp.is_none() {
        return Err(ContractError::WrongExpiration {});
    }

    // create a proposal
    let mut prop = Proposal {
        proposer: info.sender.clone(),
        title,
        description,
        start_height: env.block.height,
        expires,
        msgs,
        status: Status::Open,
        votes: Votes::yes(vote_power),
        threshold: cfg.threshold,
        total_weight: group_addr.total_weight(&deps.querier)?,
    };

    // Ensure that the incoming update config message doesn't propose
    // a threshold that would excede the sum of weights of members of
    // the multisig.
    prop.validate_update_config_msgs(deps.storage, &deps.querier)?;

    prop.update_status(&env.block);
    let id = next_id(deps.storage)?;
    PROPOSALS.save(deps.storage, id, &prop)?;

    // add the first yes vote from voter
    let ballot = Ballot {
        weight: vote_power,
        vote: Vote::Yes,
    };
    BALLOTS.save(deps.storage, (id, &info.sender), &ballot)?;

    Ok(Response::new()
        .add_attribute("action", "propose")
        .add_attribute("sender", info.sender)
        .add_attribute("proposal_id", id.to_string())
        .add_attribute("status", format!("{:?}", prop.status)))
}

pub fn execute_vote(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    proposal_id: u64,
    vote: Vote,
) -> Result<Response<Empty>, ContractError> {
    // only members of the multisig can vote
    let group_addr = GROUP_ADDRESS.load(deps.storage)?;

    // ensure proposal exists and can be voted on
    let mut prop = PROPOSALS.load(deps.storage, proposal_id)?;
    if prop.status != Status::Open {
        return Err(ContractError::NotOpen {});
    }
    if prop.expires.is_expired(&env.block) {
        return Err(ContractError::Expired {});
    }

    // Only voting members of the multisig can vote
    // Additional check if weight >= 1
    // use a snapshot of "start of proposal"
    let vote_power = group_addr
        .is_voting_member(&deps.querier, &info.sender, prop.start_height)?
        .ok_or(ContractError::Unauthorized {})?;

    // cast vote if no vote previously cast
    BALLOTS.update(deps.storage, (proposal_id, &info.sender), |bal| match bal {
        Some(_) => Err(ContractError::AlreadyVoted {}),
        None => Ok(Ballot {
            weight: vote_power,
            vote,
        }),
    })?;

    // update vote tally
    prop.votes.add_vote(vote, vote_power);
    prop.update_status(&env.block);
    PROPOSALS.save(deps.storage, proposal_id, &prop)?;

    Ok(Response::new()
        .add_attribute("action", "vote")
        .add_attribute("sender", info.sender)
        .add_attribute("proposal_id", proposal_id.to_string())
        .add_attribute("status", format!("{:?}", prop.status)))
}

pub fn execute_execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    proposal_id: u64,
) -> Result<Response, ContractError> {
    // only members can trigger this when the vote has passed
    let group_addr = GROUP_ADDRESS.load(deps.storage)?;

    let cfg = CONFIG.load(deps.storage)?;
    if cfg.only_members_execute {
        group_addr
            .is_member(&deps.querier, &info.sender, None)?
            .ok_or(ContractError::Unauthorized {})?;
    }

    let mut prop = PROPOSALS.load(deps.storage, proposal_id)?;
    // we allow execution even after the proposal "expiration" as long as all vote come in before
    // that point. If it was approved on time, it can be executed any time.
    if prop.current_status(&env.block) != Status::Passed {
        return Err(ContractError::WrongExecuteStatus {});
    }

    // set it to executed
    prop.status = Status::Executed;
    PROPOSALS.save(deps.storage, proposal_id, &prop)?;

    // dispatch all proposed messages
    Ok(Response::new()
        .add_messages(prop.msgs)
        .add_attribute("action", "execute")
        .add_attribute("sender", info.sender)
        .add_attribute("proposal_id", proposal_id.to_string()))
}

pub fn execute_close(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    proposal_id: u64,
) -> Result<Response<Empty>, ContractError> {
    // anyone can trigger this if the vote passed

    let mut prop = PROPOSALS.load(deps.storage, proposal_id)?;
    if [Status::Executed, Status::Rejected, Status::Passed]
        .iter()
        .any(|x| *x == prop.status)
    {
        return Err(ContractError::WrongCloseStatus {});
    }
    if !prop.expires.is_expired(&env.block) {
        return Err(ContractError::NotExpired {});
    }

    // set it to failed
    prop.status = Status::Rejected;
    PROPOSALS.save(deps.storage, proposal_id, &prop)?;

    Ok(Response::new()
        .add_attribute("action", "close")
        .add_attribute("sender", info.sender)
        .add_attribute("proposal_id", proposal_id.to_string()))
}

pub fn execute_membership_hook(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    _diffs: Vec<MemberDiff>,
) -> Result<Response<Empty>, ContractError> {
    // This is now a no-op
    // But we leave the authorization check as a demo
    let group_addr = GROUP_ADDRESS.load(deps.storage)?;
    if info.sender != group_addr.0 {
        return Err(ContractError::Unauthorized {});
    }

    Ok(Response::default())
}

pub fn execute_update_config(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    new_config: Config,
) -> Result<Response<Empty>, ContractError> {
    // Only contract can call this method
    if env.contract.address != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    let group_addr = GROUP_ADDRESS.load(deps.storage)?;

    let total_weight = group_addr.total_weight(&deps.querier)?;
    new_config.threshold.validate(total_weight)?;

    CONFIG.save(deps.storage, &new_config)?;

    Ok(Response::new()
        .add_attribute("action", "update_config")
        .add_attribute("sender", info.sender))
}

pub fn execute_update_cw20_token_list(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    to_add: Vec<Addr>,
    to_remove: Vec<Addr>,
) -> Result<Response<Empty>, ContractError> {
    // Only contract can call this method
    if env.contract.address != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    // Limit the number of token modifications that can occur in one
    // execution to prevent out of gas issues.
    if to_add.len() + to_remove.len() > MAX_LIMIT as usize {
        return Err(ContractError::OversizedRequest {
            size: (to_add.len() + to_remove.len()) as u64,
            max: MAX_LIMIT as u64,
        });
    }

    for token in &to_add {
        TREASURY_TOKENS.save(deps.storage, token, &Empty {})?;
    }

    for token in &to_remove {
        TREASURY_TOKENS.remove(deps.storage, token);
    }

    Ok(Response::new().add_attribute("action", "update_cw20_token_list"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Threshold {} => to_binary(&query_threshold(deps)?),
        QueryMsg::Proposal { proposal_id } => to_binary(&query_proposal(deps, env, proposal_id)?),
        QueryMsg::Vote { proposal_id, voter } => to_binary(&query_vote(deps, proposal_id, voter)?),
        QueryMsg::ListProposals { start_after, limit } => {
            to_binary(&query_list_proposals(deps, env, start_after, limit)?)
        }
        QueryMsg::ReverseProposals {
            start_before,
            limit,
        } => to_binary(&query_reverse_proposals(deps, env, start_before, limit)?),
        QueryMsg::ProposalCount {} => to_binary(&query_proposal_count(deps)),
        QueryMsg::ListVotes {
            proposal_id,
            start_after,
            limit,
        } => to_binary(&query_list_votes(deps, proposal_id, start_after, limit)?),
        QueryMsg::Voter { address } => to_binary(&query_voter(deps, address)?),
        QueryMsg::ListVoters { start_after, limit } => {
            to_binary(&query_list_voters(deps, start_after, limit)?)
        }
        QueryMsg::GetConfig {} => to_binary(&query_config(deps)?),
        QueryMsg::Tally { proposal_id } => {
            to_binary(&query_proposal_tally(deps, env, proposal_id)?)
        }
        QueryMsg::Cw20Balances { start_after, limit } => {
            to_binary(&query_cw20_balances(deps, env, start_after, limit)?)
        }
        QueryMsg::Cw20TokenList {} => to_binary(&query_cw20_token_list(deps)),
    }
}

fn query_threshold(deps: Deps) -> StdResult<ThresholdResponse> {
    let cfg = CONFIG.load(deps.storage)?;
    let group_addr = GROUP_ADDRESS.load(deps.storage)?;
    let total_weight = group_addr.total_weight(&deps.querier)?;
    Ok(cfg.threshold.to_response(total_weight))
}

fn query_proposal(deps: Deps, env: Env, id: u64) -> StdResult<ProposalResponse> {
    let prop = PROPOSALS.load(deps.storage, id)?;
    let status = prop.current_status(&env.block);
    let threshold = prop.threshold.to_response(prop.total_weight);
    Ok(ProposalResponse {
        id,
        title: prop.title,
        description: prop.description,
        proposer: prop.proposer,
        msgs: prop.msgs,
        status,
        expires: prop.expires,
        threshold,
    })
}

fn query_proposal_tally(deps: Deps, env: Env, id: u64) -> StdResult<VoteTallyResponse> {
    let prop = PROPOSALS.load(deps.storage, id)?;
    let status = prop.current_status(&env.block);
    let total_weight = prop.total_weight;
    let threshold = prop.threshold.to_response(total_weight);

    let total_votes = Uint128::from(prop.votes.total());
    let quorum = Decimal::from_ratio(total_votes, total_weight);

    Ok(VoteTallyResponse {
        status,
        threshold,
        quorum,
        total_votes,
        total_weight: Uint128::from(total_weight),
        votes: prop.votes,
    })
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    let group_address = GROUP_ADDRESS.load(deps.storage)?;
    Ok(ConfigResponse {
        config,
        group_address,
    })
}

fn query_cw20_token_list(deps: Deps) -> TokenListResponse {
    let token_list: Result<Vec<Addr>, FromUtf8Error> = TREASURY_TOKENS
        .keys_raw(deps.storage, None, None, Order::Ascending)
        .map(|token| String::from_utf8(token).map(Addr::unchecked))
        .collect();

    match token_list {
        Ok(token_list) => TokenListResponse { token_list },
        Err(_) => TokenListResponse { token_list: vec![] },
    }
}

fn query_cw20_balances(
    deps: Deps,
    env: Env,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Cw20BalancesResponse> {
    let limit = get_and_check_limit(limit, MAX_LIMIT, DEFAULT_LIMIT)? as usize;

    let start_addr = maybe_addr(deps.api, start_after)?;
    let start = start_addr.map(|addr| Bound::exclusive(addr.as_ref()));

    let cw20_balances: Vec<Cw20CoinVerified> = TREASURY_TOKENS
        .keys_raw(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|cw20_contract_address| {
            let cw20_contract_address = String::from_utf8(cw20_contract_address)
                .map(Addr::unchecked)
                .unwrap();
            let balance: BalanceResponse = deps
                .querier
                .query_wasm_smart(
                    &cw20_contract_address,
                    &Cw20QueryMsg::Balance {
                        address: env.contract.address.to_string(),
                    },
                )
                .unwrap_or(BalanceResponse {
                    balance: Uint128::zero(),
                });

            Cw20CoinVerified {
                address: cw20_contract_address,
                amount: balance.balance,
            }
        })
        .collect();

    Ok(Cw20BalancesResponse { cw20_balances })
}

fn query_list_proposals(
    deps: Deps,
    env: Env,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<ProposalListResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(Bound::exclusive_int);
    let props: StdResult<Vec<_>> = PROPOSALS
        .range_raw(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|p| map_proposal(&env.block, p))
        .collect();

    Ok(ProposalListResponse { proposals: props? })
}

fn query_proposal_count(deps: Deps) -> u64 {
    PROPOSALS
        .keys(deps.storage, None, None, Order::Descending)
        .count() as u64
}

fn query_reverse_proposals(
    deps: Deps,
    env: Env,
    start_before: Option<u64>,
    limit: Option<u32>,
) -> StdResult<ProposalListResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let end = start_before.map(Bound::exclusive_int);
    let props: StdResult<Vec<_>> = PROPOSALS
        .range_raw(deps.storage, None, end, Order::Descending)
        .take(limit)
        .map(|p| map_proposal(&env.block, p))
        .collect();

    Ok(ProposalListResponse { proposals: props? })
}

fn query_vote(deps: Deps, proposal_id: u64, voter: String) -> StdResult<VoteResponse> {
    let voter_addr = deps.api.addr_validate(&voter)?;
    let prop = BALLOTS.may_load(deps.storage, (proposal_id, &voter_addr))?;
    let vote = prop.map(|b| VoteInfo {
        voter,
        vote: b.vote,
        weight: b.weight,
    });
    Ok(VoteResponse { vote })
}

fn query_list_votes(
    deps: Deps,
    proposal_id: u64,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<VoteListResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let addr = maybe_addr(deps.api, start_after)?;
    let start = addr.map(|addr| Bound::exclusive(addr.as_ref()));

    let votes: StdResult<Vec<_>> = BALLOTS
        .prefix(proposal_id)
        .range_raw(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (voter, ballot) = item?;
            Ok(VoteInfo {
                voter: String::from_utf8(voter)?,
                vote: ballot.vote,
                weight: ballot.weight,
            })
        })
        .collect();

    Ok(VoteListResponse { votes: votes? })
}

fn query_voter(deps: Deps, voter: String) -> StdResult<VoterResponse> {
    let group_addr = GROUP_ADDRESS.load(deps.storage)?;
    let voter_addr = deps.api.addr_validate(&voter)?;
    let weight = group_addr.is_member(&deps.querier, &voter_addr, None)?;

    Ok(VoterResponse { weight })
}

fn query_list_voters(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<VoterListResponse> {
    let group_addr = GROUP_ADDRESS.load(deps.storage)?;
    let voters = group_addr
        .list_members(&deps.querier, start_after, limit)?
        .into_iter()
        .map(|member| VoterDetail {
            addr: member.addr,
            weight: member.weight,
        })
        .collect();
    Ok(VoterListResponse { voters })
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    if msg.id != INSTANTIATE_CW4_GROUP_REPLY_ID {
        return Err(ContractError::UnknownReplyId { id: msg.id });
    };
    let res = parse_reply_instantiate_data(msg);
    match res {
        Ok(res) => {
            // Get group contract
            let group_addr =
                Cw4Contract(deps.api.addr_validate(&res.contract_address).map_err(|_| {
                    ContractError::InvalidGroup {
                        addr: res.contract_address.clone(),
                    }
                })?);

            // Validate threshold
            let cfg = CONFIG.load(deps.storage)?;
            let total_weight = group_addr.total_weight(&deps.querier)?;
            cfg.threshold.validate(total_weight)?;

            // Save group address
            GROUP_ADDRESS.save(deps.storage, &group_addr)?;

            Ok(Response::new())
        }
        Err(_) => Err(ContractError::InstantiateGroupContractError {}),
    }
}
