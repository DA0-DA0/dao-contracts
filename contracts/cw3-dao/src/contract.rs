use crate::error::ContractError;
use crate::helpers::{
    get_balance, get_deposit_message, get_proposal_deposit_refund_message, get_total_supply,
    get_voting_power_at_height, map_proposal,
};
use crate::msg::{ExecuteMsg, GovTokenMsg, InstantiateMsg, ProposeMsg, QueryMsg, VoteMsg};
use crate::query::{
    ConfigResponse, Cw20BalancesResponse, ProposalListResponse, ProposalResponse,
    ThresholdResponse, TokenListResponse, VoteInfo, VoteListResponse, VoteResponse,
    VoteTallyResponse, VoterResponse,
};
use crate::state::{
    next_id, Ballot, Config, Proposal, Votes, BALLOTS, CONFIG, GOV_TOKEN, PROPOSALS,
    TREASURY_TOKENS,
};
use cosmwasm_std::{
    entry_point, to_binary, Addr, Binary, CosmosMsg, Decimal, Deps, DepsMut, Empty, Env,
    MessageInfo, Order, Reply, Response, StdResult, SubMsg, Uint128, WasmMsg,
};
use cw0::{maybe_addr, parse_reply_instantiate_data, Expiration};
use cw2::set_contract_version;
use cw20::{BalanceResponse, Cw20CoinVerified, Cw20Contract, Cw20QueryMsg, MinterResponse};
use cw3::{Status, Vote};
use cw_storage_plus::Bound;
use std::cmp::Ordering;
use std::string::FromUtf8Error;

// version info for migration info
pub const CONTRACT_NAME: &str = "crates.io:sg_dao";
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

// settings for pagination
const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;
const INSTANTIATE_GOV_TOKEN_REPLY_ID: u64 = 0;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    msg.threshold.validate()?;

    let cfg = Config {
        name: msg.name,
        description: msg.description,
        threshold: msg.threshold,
        max_voting_period: msg.max_voting_period,
        proposal_deposit: msg.proposal_deposit_amount,
        refund_failed_proposals: msg.refund_failed_proposals,
    };
    CONFIG.save(deps.storage, &cfg)?;

    let mut msgs: Vec<SubMsg> = vec![];

    match msg.gov_token {
        GovTokenMsg::InstantiateNewCw20 {
            code_id,
            label,
            msg,
        } => {
            if msg.initial_balances.is_empty() {
                return Err(ContractError::InitialBalancesError {});
            }

            let msg = WasmMsg::Instantiate {
                code_id,
                funds: vec![],
                admin: Some(env.contract.address.to_string()),
                label,
                msg: to_binary(&cw20_gov::msg::InstantiateMsg { cw20_base: cw20_base::msg::InstantiateMsg {
                    name: msg.name,
                    symbol: msg.symbol,
                    decimals: msg.decimals,
                    initial_balances: msg.initial_balances,
                    mint: Some(MinterResponse {
                        minter: env.contract.address.to_string(),
                        cap: None,
                    }),
                    marketing: msg.marketing,
                },
                    unstaking_duration: None
                })?,
            };

            let msg = SubMsg::reply_on_success(msg, INSTANTIATE_GOV_TOKEN_REPLY_ID);

            msgs.append(&mut vec![msg]);
        }
        GovTokenMsg::UseExistingCw20 { addr } => {
            let cw20_addr = Cw20Contract(
                deps.api
                    .addr_validate(&addr)
                    .map_err(|_| ContractError::InvalidCw20 { addr })?,
            );

            // Add cw20-gov token to map of TREASURY TOKENS
            TREASURY_TOKENS.save(deps.storage, &cw20_addr.addr(), &Empty {})?;

            // Save gov token
            GOV_TOKEN.save(deps.storage, &cw20_addr.addr())?;
        }
    };

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
        ExecuteMsg::Propose(ProposeMsg {
            title,
            description,
            msgs,
            latest,
        }) => execute_propose(deps, env, info, title, description, msgs, latest),
        ExecuteMsg::Vote(VoteMsg { proposal_id, vote }) => {
            execute_vote(deps, env, info, proposal_id, vote)
        }
        ExecuteMsg::Execute { proposal_id } => execute_execute(deps, env, info, proposal_id),
        ExecuteMsg::Close { proposal_id } => execute_close(deps, env, info, proposal_id),
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
    msgs: Vec<CosmosMsg<Empty>>,
    // we ignore earliest
    latest: Option<Expiration>,
) -> Result<Response<Empty>, ContractError> {
    // let {title, description, msgs, latest} = proposal;
    let cfg = CONFIG.load(deps.storage)?;
    let gov_token = GOV_TOKEN.load(deps.storage)?;

    // Only owners of the gov token can create a proposal
    let balance = get_balance(deps.as_ref(), info.sender.clone())?;
    if balance == Uint128::zero() {
        return Err(ContractError::Unauthorized {});
    }

    // Max expires also used as default
    let max_expires = cfg.max_voting_period.after(&env.block);
    let mut expires = latest.unwrap_or(max_expires);
    let comp = expires.partial_cmp(&max_expires);
    if let Some(Ordering::Greater) = comp {
        expires = max_expires;
    } else if comp.is_none() {
        return Err(ContractError::WrongExpiration {});
    }

    // Get total supply
    let total_supply = get_total_supply(deps.as_ref())?;

    // Create a proposal
    let mut prop = Proposal {
        title,
        description,
        proposer: info.sender.clone(),
        start_height: env.block.height,
        expires,
        msgs,
        status: Status::Open,
        votes: Votes {
            yes: Uint128::zero(),
            no: Uint128::zero(),
            abstain: Uint128::zero(),
            veto: Uint128::zero(),
        },
        threshold: cfg.threshold.clone(),
        total_weight: total_supply,
        deposit: cfg.proposal_deposit,
    };
    prop.update_status(&env.block);
    let id = next_id(deps.storage)?;
    PROPOSALS.save(deps.storage, id.into(), &prop)?;

    let deposit_msg = get_deposit_message(&env, &info, &cfg.proposal_deposit, &gov_token)?;

    Ok(Response::new()
        .add_messages(deposit_msg)
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
    // ensure proposal exists and can be voted on
    let mut prop = PROPOSALS.load(deps.storage, proposal_id.into())?;
    if prop.status != Status::Open {
        return Err(ContractError::NotOpen {});
    }
    if prop.expires.is_expired(&env.block) {
        return Err(ContractError::Expired {});
    }

    // Get voter balance at proposal start
    let vote_power =
        get_voting_power_at_height(deps.as_ref(), info.sender.clone(), prop.start_height)?;

    if vote_power == Uint128::zero() {
        return Err(ContractError::Unauthorized {});
    }

    // cast vote if no vote previously cast
    BALLOTS.update(
        deps.storage,
        (proposal_id.into(), &info.sender),
        |bal| match bal {
            Some(_) => Err(ContractError::AlreadyVoted {}),
            None => Ok(Ballot {
                weight: vote_power,
                vote,
            }),
        },
    )?;

    // update vote tally
    prop.votes.add_vote(vote, vote_power);
    prop.update_status(&env.block);
    PROPOSALS.save(deps.storage, proposal_id.into(), &prop)?;

    Ok(Response::new()
        .add_attribute("action", "vote")
        .add_attribute("sender", info.sender)
        .add_attribute("proposal_id", proposal_id.to_string())
        .add_attribute("status", format!("{:?}", prop.status)))
}

pub fn execute_execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    proposal_id: u64,
) -> Result<Response, ContractError> {
    let gov_token = GOV_TOKEN.load(deps.storage)?;

    // anyone can trigger this if the vote passed
    let mut prop = PROPOSALS.load(deps.storage, proposal_id.into())?;
    // we allow execution even after the proposal "expiration" as long as all vote come in before
    // that point. If it was approved on time, it can be executed any time.
    if prop.status != Status::Passed {
        return Err(ContractError::WrongExecuteStatus {});
    }

    // set it to executed
    prop.status = Status::Executed;
    PROPOSALS.save(deps.storage, proposal_id.into(), &prop)?;

    let refund_msg =
        get_proposal_deposit_refund_message(&prop.proposer, &prop.deposit, &gov_token)?;

    // dispatch all proposed messages
    Ok(Response::new()
        .add_messages(refund_msg)
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
    let gov_token = GOV_TOKEN.load(deps.storage)?;

    // anyone can trigger this if the vote passed
    let mut prop = PROPOSALS.load(deps.storage, proposal_id.into())?;
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
    PROPOSALS.save(deps.storage, proposal_id.into(), &prop)?;

    let cfg = CONFIG.load(deps.storage)?;

    let response_with_optional_refund = match cfg.refund_failed_proposals {
        Some(true) => Response::new().add_messages(get_proposal_deposit_refund_message(
            &prop.proposer,
            &prop.deposit,
            &gov_token,
        )?),
        _ => Response::new(),
    };

    Ok(response_with_optional_refund
        .add_attribute("action", "close")
        .add_attribute("sender", info.sender)
        .add_attribute("proposal_id", proposal_id.to_string()))
}

pub fn execute_update_config(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    update_config_msg: Config,
) -> Result<Response<Empty>, ContractError> {
    // Only contract can call this method
    if env.contract.address != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    update_config_msg.threshold.validate()?;

    CONFIG.save(deps.storage, &update_config_msg)?;

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
        QueryMsg::ListVotes {
            proposal_id,
            start_after,
            limit,
        } => to_binary(&query_list_votes(deps, proposal_id, start_after, limit)?),
        QueryMsg::Voter { address } => to_binary(&query_voter(deps, address)?),
        QueryMsg::Cw20Balances { start_after, limit } => {
            to_binary(&query_cw20_balances(deps, env, start_after, limit)?)
        }
        QueryMsg::GetConfig {} => to_binary(&query_config(deps)?),
        QueryMsg::Cw20TokenList {} => to_binary(&query_cw20_token_list(deps)),
        QueryMsg::Tally { proposal_id } => {
            to_binary(&query_proposal_tally(deps, env, proposal_id)?)
        }
    }
}

fn query_threshold(deps: Deps) -> StdResult<ThresholdResponse> {
    let cfg = CONFIG.load(deps.storage)?;
    let total_supply = get_total_supply(deps)?;
    Ok(cfg.threshold.to_response(total_supply))
}

fn query_proposal(deps: Deps, env: Env, id: u64) -> StdResult<ProposalResponse> {
    let prop = PROPOSALS.load(deps.storage, id.into())?;
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
        deposit_amount: prop.deposit,
    })
}

fn query_proposal_tally(deps: Deps, env: Env, id: u64) -> StdResult<VoteTallyResponse> {
    let prop = PROPOSALS.load(deps.storage, id.into())?;
    let status = prop.current_status(&env.block);
    let total_weight = prop.total_weight;
    let threshold = prop.threshold.to_response(total_weight);

    let total_votes = prop.votes.total();
    let quorum = Decimal::from_ratio(total_votes, total_weight);

    Ok(VoteTallyResponse {
        status,
        threshold,
        quorum,
        total_votes,
        total_weight,
        votes: prop.votes,
    })
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    let gov_token = GOV_TOKEN.load(deps.storage)?;
    Ok(ConfigResponse { config, gov_token })
}

fn query_cw20_token_list(deps: Deps) -> TokenListResponse {
    let token_list: Result<Vec<Addr>, FromUtf8Error> = TREASURY_TOKENS
        .keys(deps.storage, None, None, Order::Ascending)
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
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start_addr = maybe_addr(deps.api, start_after)?;
    let start = start_addr.map(|addr| Bound::exclusive(addr.as_ref()));

    let cw20_balances: Vec<Cw20CoinVerified> = TREASURY_TOKENS
        .keys(deps.storage, start, None, Order::Ascending)
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
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|p| map_proposal(&env.block, p))
        .collect();

    Ok(ProposalListResponse { proposals: props? })
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
        .range(deps.storage, None, end, Order::Descending)
        .take(limit)
        .map(|p| map_proposal(&env.block, p))
        .collect();

    Ok(ProposalListResponse { proposals: props? })
}

fn query_vote(deps: Deps, proposal_id: u64, voter: String) -> StdResult<VoteResponse> {
    let voter_addr = deps.api.addr_validate(&voter)?;
    let prop = BALLOTS.may_load(deps.storage, (proposal_id.into(), &voter_addr))?;
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
        .prefix(proposal_id.into())
        .range(deps.storage, start, None, Order::Ascending)
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
    let voter_addr = deps.api.addr_validate(&voter)?;
    let weight = get_balance(deps, voter_addr)?;

    Ok(VoterResponse {
        weight: Some(weight),
    })
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    if msg.id != INSTANTIATE_GOV_TOKEN_REPLY_ID {
        return Err(ContractError::UnknownReplyId { id: msg.id });
    };
    let res = parse_reply_instantiate_data(msg);
    match res {
        Ok(res) => {
            // Validate contract address
            let cw20_addr = deps.api.addr_validate(&res.contract_address)?;

            // Add cw20-gov token to map of TREASURY TOKENS
            TREASURY_TOKENS.save(deps.storage, &cw20_addr, &Empty {})?;

            // Save gov token
            GOV_TOKEN.save(deps.storage, &cw20_addr)?;

            Ok(Response::new())
        }
        Err(_) => Err(ContractError::InstantiateGovTokenError {}),
    }
}
