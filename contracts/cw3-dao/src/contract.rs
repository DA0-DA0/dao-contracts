use crate::error::ContractError;
use crate::helpers::{
    get_and_check_limit, get_deposit_message, get_proposal_deposit_refund_message,
    get_staked_balance, get_total_staked_supply, get_voting_power_at_height, map_proposal,
};
use crate::msg::{ExecuteMsg, GovTokenMsg, InstantiateMsg, ProposeMsg, QueryMsg, VoteMsg};
use crate::query::{
    ConfigResponse, Cw20BalancesResponse, ProposalListResponse, ProposalResponse,
    ThresholdResponse, TokenListResponse, VoteInfo, VoteListResponse, VoteResponse,
    VoteTallyResponse, VoterResponse,
};
use crate::state::{
    next_id, Ballot, Config, Proposal, Votes, BALLOTS, CONFIG, DAO_PAUSED, GOV_TOKEN, PROPOSALS,
    STAKING_CONTRACT, STAKING_CONTRACT_CODE_ID, STAKING_CONTRACT_UNSTAKING_DURATION,
    TREASURY_TOKENS,
};
use cosmwasm_std::{
    entry_point, to_binary, Addr, Binary, CosmosMsg, Decimal, Deps, DepsMut, Empty, Env,
    MessageInfo, Order, Reply, Response, StdResult, SubMsg, Uint128, WasmMsg,
};
use cw2::set_contract_version;
use cw20::{
    BalanceResponse, Cw20Coin, Cw20CoinVerified, Cw20Contract, Cw20QueryMsg, MinterResponse,
};
use cw3::{Status, Vote};
use cw_storage_plus::Bound;
use cw_utils::{maybe_addr, parse_reply_instantiate_data, Expiration};
use std::cmp::Ordering;
use std::string::FromUtf8Error;

// Version info for migration info
pub const CONTRACT_NAME: &str = "crates.io:sg_dao";
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

// Settings for pagination
const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;

// Reply IDs
const INSTANTIATE_GOV_TOKEN_REPLY_ID: u64 = 0;
const INSTANTIATE_STAKING_CONTRACT_REPLY_ID: u64 = 1;

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
        image_url: msg.image_url,
    };
    CONFIG.save(deps.storage, &cfg)?;

    let mut msgs: Vec<SubMsg> = vec![];

    match msg.gov_token {
        GovTokenMsg::InstantiateNewCw20 {
            cw20_code_id,
            stake_contract_code_id,
            label,
            initial_dao_balance,
            msg,
            unstaking_duration,
        } => {
            // Check that someone has an initial balance to be able to vote in the DAO
            if msg.initial_balances.is_empty() {
                return Err(ContractError::InitialBalancesError {});
            }

            let mut initial_balances = msg.initial_balances;

            // Check if an initial gov token balance will be created for the DAO
            if let Some(initial_dao_balance) = initial_dao_balance {
                initial_balances.push(Cw20Coin {
                    address: env.contract.address.to_string(),
                    amount: initial_dao_balance,
                });
            }

            // Save info for use in reply SubMsgs
            STAKING_CONTRACT_CODE_ID.save(deps.storage, &stake_contract_code_id)?;
            STAKING_CONTRACT_UNSTAKING_DURATION.save(deps.storage, &unstaking_duration)?;

            // Instantiate new Gov Token with DAO as admin and minter
            let msg = WasmMsg::Instantiate {
                code_id: cw20_code_id,
                funds: vec![],
                admin: Some(env.contract.address.to_string()),
                label,
                msg: to_binary(&cw20_base::msg::InstantiateMsg {
                    name: msg.name,
                    symbol: msg.symbol,
                    decimals: msg.decimals,
                    initial_balances,
                    mint: Some(MinterResponse {
                        minter: env.contract.address.to_string(),
                        cap: None,
                    }),
                    marketing: msg.marketing,
                })?,
            };

            let msg = SubMsg::reply_on_success(msg, INSTANTIATE_GOV_TOKEN_REPLY_ID);

            msgs.append(&mut vec![msg]);
        }
        GovTokenMsg::UseExistingCw20 {
            addr,
            stake_contract_code_id,
            label,
            unstaking_duration,
        } => {
            let cw20_addr = Cw20Contract(
                deps.api
                    .addr_validate(&addr)
                    .map_err(|_| ContractError::InvalidCw20 { addr })?,
            );

            // Add cw20 token to map of TREASURY TOKENS
            TREASURY_TOKENS.save(deps.storage, &cw20_addr.addr(), &Empty {})?;

            // Save gov token
            GOV_TOKEN.save(deps.storage, &cw20_addr.addr())?;

            // Instantiate staking contract with DAO as admin
            let msg = WasmMsg::Instantiate {
                code_id: stake_contract_code_id,
                funds: vec![],
                admin: Some(env.contract.address.to_string()),
                label,
                msg: to_binary(&stake_cw20::msg::InstantiateMsg {
                    admin: Some(env.contract.address),
                    unstaking_duration,
                    token_address: cw20_addr.addr(),
                })?,
            };

            let msg = SubMsg::reply_on_success(msg, INSTANTIATE_STAKING_CONTRACT_REPLY_ID);

            msgs.append(&mut vec![msg]);
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
        ExecuteMsg::PauseDAO { expiration } => execute_pause_dao(deps, env, info, expiration),
        ExecuteMsg::UpdateConfig(config) => execute_update_config(deps, env, info, config),
        ExecuteMsg::UpdateCw20TokenList { to_add, to_remove } => {
            execute_update_cw20_token_list(deps, env, info, to_add, to_remove)
        }
        ExecuteMsg::UpdateStakingContract {
            new_staking_contract,
        } => execute_update_staking_contract(deps, env, info, new_staking_contract),
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
    // Check if DAO is Paused
    let paused = DAO_PAUSED.may_load(deps.storage)?;
    if let Some(expiration) = paused {
        if !expiration.is_expired(&env.block) {
            return Err(ContractError::Paused {});
        }
    }

    let cfg = CONFIG.load(deps.storage)?;
    let gov_token = GOV_TOKEN.load(deps.storage)?;

    // Only owners of the gov token can create a proposal
    let balance = get_staked_balance(deps.as_ref(), info.sender.clone())?;
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
    let total_supply = get_total_staked_supply(deps.as_ref())?;

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
    PROPOSALS.save(deps.storage, id, &prop)?;

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
    // Check if DAO is Paused
    let paused = DAO_PAUSED.may_load(deps.storage)?;
    if let Some(expiration) = paused {
        if !expiration.is_expired(&env.block) {
            return Err(ContractError::Paused {});
        }
    }

    // Ensure proposal exists and can be voted on
    let mut prop = PROPOSALS.load(deps.storage, proposal_id)?;
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

    // Cast vote if no vote previously cast
    BALLOTS.update(deps.storage, (proposal_id, &info.sender), |bal| match bal {
        Some(_) => Err(ContractError::AlreadyVoted {}),
        None => Ok(Ballot {
            weight: vote_power,
            vote,
        }),
    })?;

    // Update vote tally
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
    // Check if DAO is Paused
    let paused = DAO_PAUSED.may_load(deps.storage)?;
    if let Some(expiration) = paused {
        if !expiration.is_expired(&env.block) {
            return Err(ContractError::Paused {});
        }
    }

    let gov_token = GOV_TOKEN.load(deps.storage)?;

    // Anyone can trigger this if the vote passed
    let mut prop = PROPOSALS.load(deps.storage, proposal_id)?;
    // We allow execution even after the proposal "expiration" as long as all vote come in before
    // that point. If it was approved on time, it can be executed any time.
    if prop.status != Status::Passed {
        return Err(ContractError::WrongExecuteStatus {});
    }

    // Set it to executed
    prop.status = Status::Executed;
    PROPOSALS.save(deps.storage, proposal_id, &prop)?;

    let refund_msg =
        get_proposal_deposit_refund_message(&prop.proposer, &prop.deposit, &gov_token)?;

    // Dispatch all proposed messages
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
    // Check if DAO is Paused
    let paused = DAO_PAUSED.may_load(deps.storage)?;
    if let Some(expiration) = paused {
        if !expiration.is_expired(&env.block) {
            return Err(ContractError::Paused {});
        }
    }

    let gov_token = GOV_TOKEN.load(deps.storage)?;

    // Anyone can trigger this if the vote passed
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

    // Set it to failed
    prop.status = Status::Rejected;
    PROPOSALS.save(deps.storage, proposal_id, &prop)?;

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

pub fn execute_pause_dao(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    expiration: Expiration,
) -> Result<Response<Empty>, ContractError> {
    // Only contract can call this method
    if env.contract.address != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    DAO_PAUSED.save(deps.storage, &expiration)?;

    Ok(Response::new()
        .add_attribute("action", "pause_dao")
        .add_attribute("expiration", expiration.to_string()))
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

pub fn execute_update_staking_contract(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    new_staking_contract: Addr,
) -> Result<Response<Empty>, ContractError> {
    // Only contract can call this method
    if env.contract.address != info.sender {
        return Err(ContractError::Unauthorized {});
    }
    let new_staking_contract = deps.api.addr_validate(new_staking_contract.as_str())?;

    // Replace the existing staking contract
    STAKING_CONTRACT.save(deps.storage, &new_staking_contract)?;

    Ok(Response::new()
        .add_attribute("action", "update_staking_contract")
        .add_attribute("new_staking_contract", new_staking_contract))
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
    let total_supply = get_total_staked_supply(deps)?;
    Ok(cfg.threshold.to_response(total_supply))
}

fn query_proposal(deps: Deps, env: Env, id: u64) -> StdResult<ProposalResponse> {
    let prop = PROPOSALS.load(deps.storage, id)?;
    let status = prop.current_status(&env.block);
    let total_supply = get_total_staked_supply(deps)?;
    let threshold = prop.threshold.to_response(total_supply);
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
    let prop = PROPOSALS.load(deps.storage, id)?;
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
    let staking_contract = STAKING_CONTRACT.load(deps.storage)?;
    Ok(ConfigResponse {
        config,
        gov_token,
        staking_contract,
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
    let limit = get_and_check_limit(limit, MAX_LIMIT, DEFAULT_LIMIT)? as usize;
    let start = start_after.map(Bound::exclusive_int);
    let props: StdResult<Vec<_>> = PROPOSALS
        .range_raw(deps.storage, start, None, Order::Ascending)
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
    let limit = get_and_check_limit(limit, MAX_LIMIT, DEFAULT_LIMIT)? as usize;
    let end = start_before.map(Bound::exclusive_int);
    let props: StdResult<Vec<_>> = PROPOSALS
        .range_raw(deps.storage, None, end, Order::Descending)
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
    let limit = get_and_check_limit(limit, MAX_LIMIT, DEFAULT_LIMIT)? as usize;
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
    let voter_addr = deps.api.addr_validate(&voter)?;
    let weight = get_staked_balance(deps, voter_addr)?;

    Ok(VoterResponse {
        weight: Some(weight),
    })
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        INSTANTIATE_GOV_TOKEN_REPLY_ID => {
            let res = parse_reply_instantiate_data(msg);
            match res {
                Ok(res) => {
                    // Validate contract address
                    let cw20_addr = deps.api.addr_validate(&res.contract_address)?;

                    // Add cw20 token to map of TREASURY TOKENS
                    TREASURY_TOKENS.save(deps.storage, &cw20_addr, &Empty {})?;

                    // Save gov token
                    GOV_TOKEN.save(deps.storage, &cw20_addr)?;

                    // Instantiate staking contract with DAO as admin
                    let code_id = STAKING_CONTRACT_CODE_ID.load(deps.storage)?;
                    let unstaking_duration =
                        STAKING_CONTRACT_UNSTAKING_DURATION.load(deps.storage)?;
                    let msg = WasmMsg::Instantiate {
                        code_id,
                        funds: vec![],
                        admin: Some(env.contract.address.to_string()),
                        label: env.contract.address.to_string(),
                        msg: to_binary(&stake_cw20::msg::InstantiateMsg {
                            admin: Some(env.contract.address),
                            unstaking_duration,
                            token_address: cw20_addr,
                        })?,
                    };
                    let msg = SubMsg::reply_on_success(msg, INSTANTIATE_STAKING_CONTRACT_REPLY_ID);

                    Ok(Response::new().add_submessage(msg))
                }
                Err(_) => Err(ContractError::InstantiateGovTokenError {}),
            }
        }
        INSTANTIATE_STAKING_CONTRACT_REPLY_ID => {
            let res = parse_reply_instantiate_data(msg);
            match res {
                Ok(res) => {
                    // Validate contract address
                    let staking_contract_addr = deps.api.addr_validate(&res.contract_address)?;

                    // Save gov token
                    STAKING_CONTRACT.save(deps.storage, &staking_contract_addr)?;

                    Ok(Response::new())
                }
                Err(_) => Err(ContractError::InstantiateGovTokenError {}),
            }
        }
        _ => Err(ContractError::UnknownReplyId { id: msg.id }),
    }
}
