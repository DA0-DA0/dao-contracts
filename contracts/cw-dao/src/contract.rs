use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, Threshold, Vote};
use crate::query::{
    ConfigResponse, ProposalListResponse, ProposalResponse, Status, ThresholdResponse, VoteInfo,
    VoteListResponse, VoteResponse, VoterResponse,
};
use crate::state::{
    next_id, parse_id, Ballot, Config, Proposal, ProposalDeposit, Votes, BALLOTS, CONFIG, PROPOSALS,
};
use cosmwasm_std::{
    entry_point, to_binary, Addr, Binary, BlockInfo, CosmosMsg, Deps, DepsMut, Empty, Env,
    MessageInfo, Order, Response, StdResult, Uint128, WasmMsg,
};
use cw0::{maybe_addr, Duration, Expiration};
use cw2::set_contract_version;
use cw20::{BalanceResponse, Cw20Contract, Cw20ExecuteMsg, Cw20QueryMsg};
use cw20_gov::msg::{BalanceAtHeightResponse, QueryMsg as Cw20GovQueryMsg};
use cw20_gov::state::TokenInfo;
use cw_storage_plus::Bound;
use std::cmp::Ordering;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:sg_dao";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

// settings for pagination
const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let cw20_addr = Cw20Contract(deps.api.addr_validate(&msg.cw20_addr).map_err(|_| {
        ContractError::InvalidCw20 {
            addr: msg.cw20_addr.clone(),
        }
    })?);

    let proposal_deposit_cw20_addr = Cw20Contract(
        deps.api
            .addr_validate(&msg.proposal_deposit_token_address)
            .map_err(|_| ContractError::InvalidCw20 {
                addr: msg.proposal_deposit_token_address.clone(),
            })?,
    );

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    msg.threshold.validate()?;

    let cfg = Config {
        threshold: msg.threshold,
        max_voting_period: msg.max_voting_period,
        cw20_addr,
        proposal_deposit: ProposalDeposit {
            amount: msg.proposal_deposit_amount,
            token_address: proposal_deposit_cw20_addr,
        },
    };
    CONFIG.save(deps.storage, &cfg)?;

    Ok(Response::default())
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
        ExecuteMsg::UpdateConfig {
            threshold,
            max_voting_period,
            proposal_deposit_amount,
            proposal_deposit_token_address,
        } => execute_update_config(
            deps,
            env,
            info,
            threshold,
            max_voting_period,
            proposal_deposit_amount,
            proposal_deposit_token_address,
        ),
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
    let cfg = CONFIG.load(deps.storage)?;

    // Only owners of the social token can create a proposal
    let balance = get_balance(deps.as_ref(), info.sender.clone())?;
    if balance == Uint128::zero() {
        return Err(ContractError::Unauthorized {});
    }

    // max expires also used as default
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

    // create a proposal
    let mut prop = Proposal {
        title,
        description,
        proposer: info.sender.clone(),
        start_height: env.block.height,
        expires,
        msgs,
        status: Status::Open,
        // votes: Votes::new(vote_power),
        votes: Votes {
            yes: Uint128::zero(),
            no: Uint128::zero(),
            abstain: Uint128::zero(),
            veto: Uint128::zero(),
        },
        threshold: cfg.threshold.clone(),
        total_weight: total_supply,
        deposit: cfg.proposal_deposit.clone(),
    };
    prop.update_status(&env.block);
    let id = next_id(deps.storage)?;
    PROPOSALS.save(deps.storage, id.into(), &prop)?;

    let deposit_msg = get_deposit_message(&env, &info, &cfg.proposal_deposit)?;

    Ok(Response::new()
        .add_messages(deposit_msg)
        .add_attribute("action", "propose")
        .add_attribute("sender", info.sender)
        .add_attribute("proposal_id", id.to_string())
        .add_attribute("status", format!("{:?}", prop.status)))
}

fn get_deposit_message(
    env: &Env,
    info: &MessageInfo,
    config: &ProposalDeposit,
) -> StdResult<Vec<CosmosMsg>> {
    if config.amount == Uint128::zero() {
        return Ok(vec![]);
    }
    let transfer_cw20_msg = Cw20ExecuteMsg::TransferFrom {
        owner: info.sender.clone().into(),
        recipient: env.contract.address.clone().into(),
        amount: config.amount,
    };
    let exec_cw20_transfer = WasmMsg::Execute {
        contract_addr: config.token_address.addr().into(),
        msg: to_binary(&transfer_cw20_msg)?,
        funds: vec![],
    };
    let cw20_transfer_cosmos_msg: CosmosMsg = exec_cw20_transfer.into();
    Ok(vec![cw20_transfer_cosmos_msg])
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
    let vote_power = get_balance_at_height(deps.as_ref(), info.sender.clone(), prop.start_height)?;

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

    let refund_msg = get_proposal_deposit_refund_message(&prop.proposer, &prop.deposit)?;

    // dispatch all proposed messages
    Ok(Response::new()
        .add_messages(refund_msg)
        .add_messages(prop.msgs)
        .add_attribute("action", "execute")
        .add_attribute("sender", info.sender)
        .add_attribute("proposal_id", proposal_id.to_string()))
}

fn get_proposal_deposit_refund_message(
    proposer: &Addr,
    config: &ProposalDeposit,
) -> StdResult<Vec<CosmosMsg>> {
    if config.amount == Uint128::zero() {
        return Ok(vec![]);
    }
    let transfer_cw20_msg = Cw20ExecuteMsg::Transfer {
        recipient: proposer.into(),
        amount: config.amount,
    };
    let exec_cw20_transfer = WasmMsg::Execute {
        contract_addr: config.token_address.addr().into(),
        msg: to_binary(&transfer_cw20_msg)?,
        funds: vec![],
    };
    let cw20_transfer_cosmos_msg: CosmosMsg = exec_cw20_transfer.into();
    Ok(vec![cw20_transfer_cosmos_msg])
}

pub fn execute_close(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    proposal_id: u64,
) -> Result<Response<Empty>, ContractError> {
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

    Ok(Response::new()
        .add_attribute("action", "close")
        .add_attribute("sender", info.sender)
        .add_attribute("proposal_id", proposal_id.to_string()))
}

pub fn execute_update_config(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    threshold: Threshold,
    max_voting_period: Duration,
    proposal_deposit_amount: Uint128,
    proposal_deposit_token_address: String,
) -> Result<Response<Empty>, ContractError> {
    // Only contract can call this method
    if env.contract.address != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    threshold.validate()?;

    let proposal_deposit_cw20_addr = Cw20Contract(
        deps.api
            .addr_validate(&proposal_deposit_token_address)
            .map_err(|_| ContractError::InvalidCw20 {
                addr: proposal_deposit_token_address.clone(),
            })?,
    );

    CONFIG.update(deps.storage, |mut exists| -> StdResult<_> {
        exists.threshold = threshold;
        exists.max_voting_period = max_voting_period;
        exists.proposal_deposit = ProposalDeposit {
            amount: proposal_deposit_amount,
            token_address: proposal_deposit_cw20_addr,
        };
        Ok(exists)
    })?;

    Ok(Response::new()
        .add_attribute("action", "update_config")
        .add_attribute("sender", info.sender))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Threshold {} => to_binary(&query_threshold(deps)?),
        QueryMsg::Proposal { proposal_id } => to_binary(&query_proposal(deps, env, proposal_id)?),
        QueryMsg::Vote { proposal_id, voter } => to_binary(&query_vote(deps, proposal_id, voter)?),
        QueryMsg::ListProposals { start_after, limit } => {
            to_binary(&list_proposals(deps, env, start_after, limit)?)
        }
        QueryMsg::ReverseProposals {
            start_before,
            limit,
        } => to_binary(&reverse_proposals(deps, env, start_before, limit)?),
        QueryMsg::ListVotes {
            proposal_id,
            start_after,
            limit,
        } => to_binary(&list_votes(deps, proposal_id, start_after, limit)?),
        QueryMsg::Voter { address } => to_binary(&query_voter(deps, address)?),
        QueryMsg::GetConfig {} => to_binary(&query_config(deps)?),
    }
}

fn get_total_supply(deps: Deps) -> StdResult<Uint128> {
    let cfg = CONFIG.load(deps.storage)?;

    // Get total supply
    let token_info: TokenInfo = deps
        .querier
        .query_wasm_smart(cfg.cw20_addr.addr(), &Cw20QueryMsg::TokenInfo {})?;
    Ok(token_info.total_supply)
}

fn get_balance(deps: Deps, address: Addr) -> StdResult<Uint128> {
    let cfg = CONFIG.load(deps.storage)?;
    // Get total supply
    let balance: BalanceResponse = deps
        .querier
        .query_wasm_smart(
            cfg.cw20_addr.addr(),
            &Cw20QueryMsg::Balance {
                address: address.to_string(),
            },
        )
        .unwrap_or(BalanceResponse {
            balance: Uint128::zero(),
        });
    Ok(balance.balance)
}

fn get_balance_at_height(deps: Deps, address: Addr, height: u64) -> StdResult<Uint128> {
    let cfg = CONFIG.load(deps.storage)?;
    // Get total supply
    let balance: BalanceAtHeightResponse = deps
        .querier
        .query_wasm_smart(
            cfg.cw20_addr.addr(),
            &Cw20GovQueryMsg::BalanceAtHeight {
                address: address.to_string(),
                height,
            },
        )
        .unwrap_or(BalanceAtHeightResponse {
            balance: Uint128::zero(),
            height: 0,
        });
    Ok(balance.balance)
}

fn query_threshold(deps: Deps) -> StdResult<ThresholdResponse> {
    let cfg = CONFIG.load(deps.storage)?;
    let total_supply = get_total_supply(deps)?;
    Ok(cfg.threshold.to_response(total_supply))
}

fn query_proposal(deps: Deps, env: Env, id: u64) -> StdResult<ProposalResponse> {
    let prop = PROPOSALS.load(deps.storage, id.into())?;
    let status = prop.current_status(&env.block);
    let total_supply = get_total_supply(deps)?;
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
        deposit_amount: prop.deposit.amount,
        deposit_token_address: prop.deposit.token_address.addr(),
    })
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse { config })
}

fn list_proposals(
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

fn reverse_proposals(
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

fn map_proposal(
    block: &BlockInfo,
    item: StdResult<(Vec<u8>, Proposal)>,
) -> StdResult<ProposalResponse> {
    let (key, prop) = item?;
    let status = prop.current_status(block);
    let threshold = prop.threshold.to_response(prop.total_weight);
    Ok(ProposalResponse {
        id: parse_id(&key)?,
        title: prop.title,
        description: prop.description,
        proposer: prop.proposer,
        msgs: prop.msgs,
        status,
        expires: prop.expires,
        threshold,
        deposit_amount: prop.deposit.amount,
        deposit_token_address: prop.deposit.token_address.addr(),
    })
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

fn list_votes(
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

#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::{mock_env, MockApi, MockStorage};
    use cosmwasm_std::{coin, coins, Addr, BankMsg, Coin, Decimal, Timestamp, WasmMsg};
    use cw0::Duration;
    use cw2::{query_contract_info, ContractVersion};
    use cw20::Cw20Coin;
    use cw_multi_test::{next_block, App, BankKeeper, Contract, ContractWrapper, Executor};

    use super::*;
    use crate::msg::Threshold;

    const OWNER: &str = "admin0001";
    const VOTER1: &str = "voter0001";
    const VOTER2: &str = "voter0002";
    const VOTER3: &str = "voter0003";
    const SOMEBODY: &str = "somebody";
    const POWER_VOTER: &str = "power-voter";

    const NATIVE_TOKEN_DENOM: &str = "ustars";
    const INITIAL_BALANCE: u128 = 2000000;

    pub fn contract_dao() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(
            crate::contract::execute,
            crate::contract::instantiate,
            crate::contract::query,
        );
        Box::new(contract)
    }

    pub fn contract_cw20_gov() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(
            cw20_gov::contract::execute,
            cw20_gov::contract::instantiate,
            cw20_gov::contract::query,
        );
        Box::new(contract)
    }

    fn mock_app() -> App {
        let env = mock_env();
        let api = MockApi::default();
        let bank = BankKeeper::new();

        App::new(api, env.block, bank, MockStorage::new())
    }

    // uploads code and returns address of cw20 contract
    fn instantiate_cw20(app: &mut App) -> Addr {
        let cw20_id = app.store_code(contract_cw20_gov());
        let msg = cw20_gov::msg::InstantiateMsg {
            name: String::from("Test"),
            symbol: String::from("TEST"),
            decimals: 6,
            initial_balances: vec![
                Cw20Coin {
                    address: OWNER.to_string(),
                    amount: Uint128::new(INITIAL_BALANCE),
                },
                Cw20Coin {
                    address: VOTER1.to_string(),
                    amount: Uint128::new(INITIAL_BALANCE),
                },
                Cw20Coin {
                    address: VOTER2.to_string(),
                    amount: Uint128::new(INITIAL_BALANCE),
                },
                Cw20Coin {
                    address: VOTER3.to_string(),
                    amount: Uint128::new(INITIAL_BALANCE * 2),
                },
                Cw20Coin {
                    address: POWER_VOTER.to_string(),
                    amount: Uint128::new(INITIAL_BALANCE * 5),
                },
            ],
            mint: None,
            marketing: None,
        };
        app.instantiate_contract(cw20_id, Addr::unchecked(OWNER), &msg, &[], "cw20", None)
            .unwrap()
    }

    fn instantiate_dao(
        app: &mut App,
        cw20: Addr,
        threshold: Threshold,
        max_voting_period: Duration,
    ) -> Addr {
        let flex_id = app.store_code(contract_dao());
        let msg = crate::msg::InstantiateMsg {
            cw20_addr: cw20.to_string(),
            threshold,
            max_voting_period,
            proposal_deposit_amount: Uint128::zero(),
            proposal_deposit_token_address: cw20.to_string(),
        };
        app.instantiate_contract(flex_id, Addr::unchecked(OWNER), &msg, &[], "flex", None)
            .unwrap()
    }

    fn setup_test_case(
        app: &mut App,
        threshold: Threshold,
        max_voting_period: Duration,
        init_funds: Vec<Coin>,
    ) -> (Addr, Addr) {
        // 1. Instantiate Social Token Contract
        let cw20_addr = instantiate_cw20(app);
        app.update_block(next_block);

        // 2. Set up Multisig backed by this group
        let dao_addr = instantiate_dao(app, cw20_addr.clone(), threshold, max_voting_period);
        app.update_block(next_block);

        // Bonus: set some funds on the multisig contract for future proposals
        if !init_funds.is_empty() {
            app.init_bank_balance(&dao_addr, init_funds).unwrap();
        }
        (dao_addr, cw20_addr)
    }

    fn proposal_info() -> (Vec<CosmosMsg<Empty>>, String, String) {
        let bank_msg = BankMsg::Send {
            to_address: SOMEBODY.into(),
            amount: coins(1, NATIVE_TOKEN_DENOM),
        };
        let msgs = vec![bank_msg.into()];
        let title = "Pay somebody".to_string();
        let description = "Do I pay her?".to_string();
        (msgs, title, description)
    }

    fn pay_somebody_proposal() -> ExecuteMsg {
        let (msgs, title, description) = proposal_info();
        ExecuteMsg::Propose {
            title,
            description,
            msgs,
            latest: None,
        }
    }

    #[test]
    fn test_instantiate_works() {
        let mut app = mock_app();

        // make a simple group
        let cw20_addr = instantiate_cw20(&mut app);
        let flex_id = app.store_code(contract_dao());

        let max_voting_period = Duration::Time(1234567);

        // Total weight less than required weight not allowed
        let instantiate_msg = InstantiateMsg {
            cw20_addr: cw20_addr.to_string(),
            threshold: Threshold::AbsolutePercentage {
                percentage: Decimal::percent(101),
            },
            max_voting_period,
            proposal_deposit_amount: Uint128::zero(),
            proposal_deposit_token_address: cw20_addr.to_string(),
        };
        let err = app
            .instantiate_contract(
                flex_id,
                Addr::unchecked(OWNER),
                &instantiate_msg,
                &[],
                "high required weight",
                None,
            )
            .unwrap_err();
        assert_eq!(
            ContractError::UnreachableThreshold {},
            err.downcast().unwrap()
        );

        // All valid
        let instantiate_msg = InstantiateMsg {
            cw20_addr: cw20_addr.to_string(),
            threshold: Threshold::ThresholdQuorum {
                threshold: Decimal::percent(51),
                quorum: Decimal::percent(10),
            },
            max_voting_period,
            proposal_deposit_amount: Uint128::zero(),
            proposal_deposit_token_address: cw20_addr.to_string(),
        };
        let dao_addr = app
            .instantiate_contract(
                flex_id,
                Addr::unchecked(OWNER),
                &instantiate_msg,
                &[],
                "all good",
                None,
            )
            .unwrap();

        // Verify contract version set properly
        let version = query_contract_info(&app, dao_addr.clone()).unwrap();
        assert_eq!(
            ContractVersion {
                contract: CONTRACT_NAME.to_string(),
                version: CONTRACT_VERSION.to_string(),
            },
            version,
        );
    }

    #[test]
    fn test_propose_works() {
        let mut app = mock_app();

        let voting_period = Duration::Time(2000000);
        let threshold = Threshold::ThresholdQuorum {
            threshold: Decimal::percent(51),
            quorum: Decimal::percent(10),
        };
        let (dao_addr, _cw20_addr) = setup_test_case(
            &mut app,
            threshold,
            voting_period,
            coins(100, NATIVE_TOKEN_DENOM),
        );

        let proposal = pay_somebody_proposal();
        // Only voters with a social token balance can propose
        let err = app
            .execute_contract(Addr::unchecked(SOMEBODY), dao_addr.clone(), &proposal, &[])
            .unwrap_err();
        assert_eq!(ContractError::Unauthorized {}, err.downcast().unwrap());

        // Wrong expiration option fails
        let msgs = match proposal.clone() {
            ExecuteMsg::Propose { msgs, .. } => msgs,
            _ => panic!("Wrong variant"),
        };
        let proposal_wrong_exp = ExecuteMsg::Propose {
            title: "Rewarding somebody".to_string(),
            description: "Do we reward her?".to_string(),
            msgs,
            latest: Some(Expiration::AtHeight(123456)),
        };
        let err = app
            .execute_contract(
                Addr::unchecked(OWNER),
                dao_addr.clone(),
                &proposal_wrong_exp,
                &[],
            )
            .unwrap_err();
        assert_eq!(ContractError::WrongExpiration {}, err.downcast().unwrap());

        // Proposal from voter works
        let res = app
            .execute_contract(Addr::unchecked(VOTER3), dao_addr.clone(), &proposal, &[])
            .unwrap();
        assert_eq!(
            res.custom_attrs(1),
            [
                ("action", "propose"),
                ("sender", VOTER3),
                ("proposal_id", "1"),
                ("status", "Open"),
            ],
        );
    }

    fn get_tally(app: &App, dao_addr: &str, proposal_id: u64) -> Uint128 {
        // Get all the voters on the proposal
        let voters = QueryMsg::ListVotes {
            proposal_id,
            start_after: None,
            limit: None,
        };
        let votes: VoteListResponse = app.wrap().query_wasm_smart(dao_addr, &voters).unwrap();
        // Sum the weights of the Yes votes to get the tally
        votes
            .votes
            .iter()
            .filter(|&v| v.vote == Vote::Yes)
            .map(|v| v.weight)
            .sum()
    }

    fn expire(voting_period: Duration) -> impl Fn(&mut BlockInfo) {
        move |block: &mut BlockInfo| {
            match voting_period {
                Duration::Time(duration) => block.time = block.time.plus_seconds(duration + 1),
                Duration::Height(duration) => block.height += duration + 1,
            };
        }
    }

    fn unexpire(voting_period: Duration) -> impl Fn(&mut BlockInfo) {
        move |block: &mut BlockInfo| {
            match voting_period {
                Duration::Time(duration) => {
                    block.time =
                        Timestamp::from_nanos(block.time.nanos() - (duration * 1_000_000_000));
                }
                Duration::Height(duration) => block.height -= duration,
            };
        }
    }

    #[test]
    fn test_proposal_queries() {
        let mut app = mock_app();

        let voting_period = Duration::Time(2000000);
        let threshold = Threshold::ThresholdQuorum {
            threshold: Decimal::percent(51),
            quorum: Decimal::percent(10),
        };
        let (dao_addr, cw20_addr) = setup_test_case(
            &mut app,
            threshold,
            voting_period,
            coins(100, NATIVE_TOKEN_DENOM),
        );

        // create proposal with 1 vote power
        let proposal = pay_somebody_proposal();
        let res = app
            .execute_contract(Addr::unchecked(VOTER1), dao_addr.clone(), &proposal, &[])
            .unwrap();
        let proposal_id1: u64 = res.custom_attrs(1)[2].value.parse().unwrap();

        // another proposal
        app.update_block(next_block);
        let proposal = pay_somebody_proposal();
        let res = app
            .execute_contract(Addr::unchecked(VOTER3), dao_addr.clone(), &proposal, &[])
            .unwrap();
        let proposal_id2: u64 = res.custom_attrs(1)[2].value.parse().unwrap();

        // Imediately passes on yes vote
        let yes_vote = ExecuteMsg::Vote {
            proposal_id: proposal_id2.clone(),
            vote: Vote::Yes,
        };
        let res = app.execute_contract(Addr::unchecked(VOTER3), dao_addr.clone(), &yes_vote, &[]);
        assert!(res.is_ok());

        // expire them both
        app.update_block(expire(voting_period));

        // add one more open proposal, 2 votes
        let proposal = pay_somebody_proposal();
        let res = app
            .execute_contract(Addr::unchecked(VOTER2), dao_addr.clone(), &proposal, &[])
            .unwrap();
        let proposal_id3: u64 = res.custom_attrs(1)[2].value.parse().unwrap();
        let proposed_at = app.block_info();

        // next block, let's query them all... make sure status is properly updated (1 should be rejected in query)
        app.update_block(next_block);
        let list_query = QueryMsg::ListProposals {
            start_after: None,
            limit: None,
        };
        let res: ProposalListResponse =
            app.wrap().query_wasm_smart(&dao_addr, &list_query).unwrap();
        assert_eq!(3, res.proposals.len());

        // check the id and status are properly set
        let info: Vec<_> = res.proposals.iter().map(|p| (p.id, p.status)).collect();
        let expected_info = vec![
            (proposal_id1, Status::Rejected),
            (proposal_id2, Status::Passed),
            (proposal_id3, Status::Open),
        ];
        assert_eq!(expected_info, info);

        // ensure the common features are set
        let (expected_msgs, expected_title, expected_description) = proposal_info();
        for prop in res.proposals {
            assert_eq!(prop.title, expected_title);
            assert_eq!(prop.description, expected_description);
            assert_eq!(prop.msgs, expected_msgs);
        }

        // reverse query can get just proposal_id3
        let list_query = QueryMsg::ReverseProposals {
            start_before: None,
            limit: Some(1),
        };
        let res: ProposalListResponse =
            app.wrap().query_wasm_smart(&dao_addr, &list_query).unwrap();
        assert_eq!(1, res.proposals.len());

        let (msgs, title, description) = proposal_info();
        let expected = ProposalResponse {
            id: proposal_id3,
            title,
            description,
            proposer: Addr::unchecked(VOTER2),
            msgs,
            expires: voting_period.after(&proposed_at),
            status: Status::Open,
            threshold: ThresholdResponse::ThresholdQuorum {
                threshold: Decimal::percent(51),
                quorum: Decimal::percent(10),
                total_weight: Uint128::new(20000000),
            },
            deposit_amount: Uint128::zero(),
            deposit_token_address: cw20_addr,
        };
        assert_eq!(&expected, &res.proposals[0]);
    }

    #[test]
    fn test_vote_works() {
        let mut app = mock_app();

        let voting_period = Duration::Time(2000000);
        let threshold = Threshold::ThresholdQuorum {
            threshold: Decimal::percent(51),
            quorum: Decimal::percent(10),
        };
        let (dao_addr, _cw20_addr) = setup_test_case(
            &mut app,
            threshold,
            voting_period,
            coins(100, NATIVE_TOKEN_DENOM),
        );

        // create proposal with 0 vote power
        let proposal = pay_somebody_proposal();
        let res = app
            .execute_contract(Addr::unchecked(OWNER), dao_addr.clone(), &proposal, &[])
            .unwrap();

        // Get the proposal id from the logs
        let proposal_id: u64 = res.custom_attrs(1)[2].value.parse().unwrap();

        // Owner votes
        let yes_vote = ExecuteMsg::Vote {
            proposal_id,
            vote: Vote::Yes,
        };
        let res = app.execute_contract(Addr::unchecked(OWNER), dao_addr.clone(), &yes_vote, &[]);
        assert!(res.is_ok());

        // Owner cannot vote (again)
        let yes_vote = ExecuteMsg::Vote {
            proposal_id,
            vote: Vote::Yes,
        };
        let err = app
            .execute_contract(Addr::unchecked(OWNER), dao_addr.clone(), &yes_vote, &[])
            .unwrap_err();
        assert_eq!(ContractError::AlreadyVoted {}, err.downcast().unwrap());

        // Only voters can vote
        let err = app
            .execute_contract(Addr::unchecked(SOMEBODY), dao_addr.clone(), &yes_vote, &[])
            .unwrap_err();
        assert_eq!(ContractError::Unauthorized {}, err.downcast().unwrap());

        // But voter1 can
        let res = app
            .execute_contract(Addr::unchecked(VOTER1), dao_addr.clone(), &yes_vote, &[])
            .unwrap();
        assert_eq!(
            res.custom_attrs(1),
            [
                ("action", "vote"),
                ("sender", VOTER1),
                ("proposal_id", proposal_id.to_string().as_str()),
                ("status", "Open"),
            ],
        );

        // No/Veto votes have no effect on the tally
        // Compute the current tally
        let tally = get_tally(&app, dao_addr.as_ref(), proposal_id);
        assert_eq!(tally, Uint128::new(4000000));

        // Cast a No vote
        let no_vote = ExecuteMsg::Vote {
            proposal_id,
            vote: Vote::No,
        };
        let _ = app
            .execute_contract(Addr::unchecked(VOTER2), dao_addr.clone(), &no_vote, &[])
            .unwrap();

        // Cast a Veto vote
        let veto_vote = ExecuteMsg::Vote {
            proposal_id,
            vote: Vote::Veto,
        };
        let _ = app
            .execute_contract(Addr::unchecked(VOTER3), dao_addr.clone(), &veto_vote, &[])
            .unwrap();

        // Tally unchanged
        assert_eq!(tally, get_tally(&app, dao_addr.as_ref(), proposal_id));

        let err = app
            .execute_contract(Addr::unchecked(VOTER3), dao_addr.clone(), &yes_vote, &[])
            .unwrap_err();
        assert_eq!(ContractError::AlreadyVoted {}, err.downcast().unwrap());

        // Expired proposals cannot be voted
        app.update_block(expire(voting_period));
        let err = app
            .execute_contract(Addr::unchecked(VOTER1), dao_addr.clone(), &yes_vote, &[])
            .unwrap_err();
        assert_eq!(ContractError::Expired {}, err.downcast().unwrap());
        app.update_block(unexpire(voting_period));

        // Power voter supports it, so it passes
        let res = app
            .execute_contract(
                Addr::unchecked(POWER_VOTER),
                dao_addr.clone(),
                &yes_vote,
                &[],
            )
            .unwrap();

        assert_eq!(
            res.custom_attrs(1),
            [
                ("action", "vote"),
                ("sender", POWER_VOTER),
                ("proposal_id", proposal_id.to_string().as_str()),
                ("status", "Passed"),
            ],
        );

        // non-Open proposals cannot be voted
        let err = app
            .execute_contract(Addr::unchecked(VOTER1), dao_addr.clone(), &yes_vote, &[])
            .unwrap_err();
        assert_eq!(ContractError::NotOpen {}, err.downcast().unwrap());

        // query individual votes
        let voter = OWNER.into();
        let vote: VoteResponse = app
            .wrap()
            .query_wasm_smart(&dao_addr, &QueryMsg::Vote { proposal_id, voter })
            .unwrap();
        assert_eq!(
            vote.vote.unwrap(),
            VoteInfo {
                voter: OWNER.into(),
                vote: Vote::Yes,
                weight: Uint128::new(2000000)
            }
        );

        // nay sayer
        let voter = VOTER2.into();
        let vote: VoteResponse = app
            .wrap()
            .query_wasm_smart(&dao_addr, &QueryMsg::Vote { proposal_id, voter })
            .unwrap();
        assert_eq!(
            vote.vote.unwrap(),
            VoteInfo {
                voter: VOTER2.into(),
                vote: Vote::No,
                weight: Uint128::new(2000000),
            }
        );

        // non-voter
        let voter = SOMEBODY.into();
        let vote: VoteResponse = app
            .wrap()
            .query_wasm_smart(&dao_addr, &QueryMsg::Vote { proposal_id, voter })
            .unwrap();
        assert!(vote.vote.is_none());
    }

    #[test]
    fn test_execute_works() {
        let mut app = mock_app();

        let voting_period = Duration::Time(2000000);
        let threshold = Threshold::ThresholdQuorum {
            threshold: Decimal::percent(10),
            quorum: Decimal::percent(10),
        };
        let (dao_addr, _cw20_addr) = setup_test_case(
            &mut app,
            threshold,
            voting_period,
            coins(10, NATIVE_TOKEN_DENOM),
        );

        // ensure we have cash to cover the proposal
        let contract_bal = app
            .wrap()
            .query_balance(&dao_addr, NATIVE_TOKEN_DENOM)
            .unwrap();
        assert_eq!(contract_bal, coin(10, NATIVE_TOKEN_DENOM));

        // create proposal with 0 vote power
        let proposal = pay_somebody_proposal();
        let res = app
            .execute_contract(Addr::unchecked(OWNER), dao_addr.clone(), &proposal, &[])
            .unwrap();

        // Get the proposal id from the logs
        let proposal_id: u64 = res.custom_attrs(1)[2].value.parse().unwrap();

        // Only Passed can be executed
        let execution = ExecuteMsg::Execute { proposal_id };
        let err = app
            .execute_contract(Addr::unchecked(OWNER), dao_addr.clone(), &execution, &[])
            .unwrap_err();
        assert_eq!(
            ContractError::WrongExecuteStatus {},
            err.downcast().unwrap()
        );

        // Vote it, so it passes
        let vote = ExecuteMsg::Vote {
            proposal_id,
            vote: Vote::Yes,
        };
        let res = app
            .execute_contract(Addr::unchecked(VOTER3), dao_addr.clone(), &vote, &[])
            .unwrap();
        assert_eq!(
            res.custom_attrs(1),
            [
                ("action", "vote"),
                ("sender", VOTER3),
                ("proposal_id", proposal_id.to_string().as_str()),
                ("status", "Passed"),
            ],
        );

        // In passing: Try to close Passed fails
        let closing = ExecuteMsg::Close { proposal_id };
        let err = app
            .execute_contract(Addr::unchecked(OWNER), dao_addr.clone(), &closing, &[])
            .unwrap_err();
        assert_eq!(ContractError::WrongCloseStatus {}, err.downcast().unwrap());

        // Execute works. Anybody can execute Passed proposals
        let res = app
            .execute_contract(Addr::unchecked(SOMEBODY), dao_addr.clone(), &execution, &[])
            .unwrap();
        assert_eq!(
            res.custom_attrs(1),
            [
                ("action", "execute"),
                ("sender", SOMEBODY),
                ("proposal_id", proposal_id.to_string().as_str()),
            ],
        );

        // verify money was transfered
        let some_bal = app
            .wrap()
            .query_balance(SOMEBODY, NATIVE_TOKEN_DENOM)
            .unwrap();
        assert_eq!(some_bal, coin(1, NATIVE_TOKEN_DENOM));
        let contract_bal = app
            .wrap()
            .query_balance(&dao_addr, NATIVE_TOKEN_DENOM)
            .unwrap();
        assert_eq!(contract_bal, coin(9, NATIVE_TOKEN_DENOM));

        // In passing: Try to close Executed fails
        let err = app
            .execute_contract(Addr::unchecked(OWNER), dao_addr, &closing, &[])
            .unwrap_err();
        assert_eq!(ContractError::WrongCloseStatus {}, err.downcast().unwrap());
    }

    #[test]
    fn test_close_works() {
        let mut app = mock_app();

        let voting_period = Duration::Height(2000000);
        let threshold = Threshold::ThresholdQuorum {
            threshold: Decimal::percent(51),
            quorum: Decimal::percent(10),
        };
        let (dao_addr, _cw20_addr) = setup_test_case(
            &mut app,
            threshold,
            voting_period,
            coins(10, NATIVE_TOKEN_DENOM),
        );

        // create proposal with 0 vote power
        let proposal = pay_somebody_proposal();
        let res = app
            .execute_contract(Addr::unchecked(OWNER), dao_addr.clone(), &proposal, &[])
            .unwrap();

        // Get the proposal id from the logs
        let proposal_id: u64 = res.custom_attrs(1)[2].value.parse().unwrap();

        // Non-expired proposals cannot be closed
        let closing = ExecuteMsg::Close { proposal_id };
        let err = app
            .execute_contract(Addr::unchecked(SOMEBODY), dao_addr.clone(), &closing, &[])
            .unwrap_err();
        assert_eq!(ContractError::NotExpired {}, err.downcast().unwrap());

        // Expired proposals can be closed
        app.update_block(expire(voting_period));
        let res = app
            .execute_contract(Addr::unchecked(SOMEBODY), dao_addr.clone(), &closing, &[])
            .unwrap();
        assert_eq!(
            res.custom_attrs(1),
            [
                ("action", "close"),
                ("sender", SOMEBODY),
                ("proposal_id", proposal_id.to_string().as_str()),
            ],
        );

        // Trying to close it again fails
        let closing = ExecuteMsg::Close { proposal_id };
        let err = app
            .execute_contract(Addr::unchecked(SOMEBODY), dao_addr, &closing, &[])
            .unwrap_err();
        assert_eq!(ContractError::WrongCloseStatus {}, err.downcast().unwrap());
    }

    #[test]
    fn quorum_enforced_even_if_absolute_threshold_met() {
        let mut app = mock_app();

        // 33% required for quora, which is 5 of the initial 15
        // 50% yes required to pass early (8 of the initial 15)
        let voting_period = Duration::Time(20000);
        let (dao_addr, _cw20_addr) = setup_test_case(
            &mut app,
            // note that 60% yes is not enough to pass without 20% no as well
            Threshold::ThresholdQuorum {
                threshold: Decimal::percent(50),
                quorum: Decimal::percent(80),
            },
            voting_period,
            coins(10, NATIVE_TOKEN_DENOM),
        );

        // create proposal
        let proposal = pay_somebody_proposal();
        let res = app
            .execute_contract(Addr::unchecked(VOTER1), dao_addr.clone(), &proposal, &[])
            .unwrap();

        // Get the proposal id from the logs
        let proposal_id: u64 = res.custom_attrs(1)[2].value.parse().unwrap();
        let prop_status = |app: &App| -> Status {
            let query_prop = QueryMsg::Proposal { proposal_id };
            let prop: ProposalResponse =
                app.wrap().query_wasm_smart(&dao_addr, &query_prop).unwrap();
            prop.status
        };
        assert_eq!(prop_status(&app), Status::Open);
        app.update_block(|block| block.height += 3);

        // reach 60% of yes votes, not enough to pass early (or late)
        let yes_vote = ExecuteMsg::Vote {
            proposal_id,
            vote: Vote::Yes,
        };
        app.execute_contract(Addr::unchecked(VOTER1), dao_addr.clone(), &yes_vote, &[])
            .unwrap();
        app.execute_contract(Addr::unchecked(VOTER2), dao_addr.clone(), &yes_vote, &[])
            .unwrap();
        app.execute_contract(Addr::unchecked(VOTER3), dao_addr.clone(), &yes_vote, &[])
            .unwrap();
        app.execute_contract(Addr::unchecked(OWNER), dao_addr.clone(), &yes_vote, &[])
            .unwrap();

        // 9 of 15 is 60% absolute threshold, but less than 12 (80% quorum needed)
        assert_eq!(prop_status(&app), Status::Open);

        // add 3 weight no vote and we hit quorum and this passes
        let no_vote = ExecuteMsg::Vote {
            proposal_id,
            vote: Vote::No,
        };
        app.execute_contract(
            Addr::unchecked(POWER_VOTER),
            dao_addr.clone(),
            &no_vote,
            &[],
        )
        .unwrap();
        assert_eq!(prop_status(&app), Status::Passed);
    }

    #[test]
    fn test_update_config() {
        let mut app = mock_app();

        let voting_period = Duration::Time(2000000);
        let threshold = Threshold::ThresholdQuorum {
            threshold: Decimal::percent(20),
            quorum: Decimal::percent(10),
        };
        let (dao_addr, cw20_addr) = setup_test_case(
            &mut app,
            threshold,
            voting_period,
            coins(100, NATIVE_TOKEN_DENOM),
        );

        // nobody can call call update contract method
        let new_threshold = Threshold::ThresholdQuorum {
            threshold: Decimal::percent(51),
            quorum: Decimal::percent(10),
        };
        let new_voting_period = Duration::Time(5000000);
        let new_proposal_deposit_amount = Uint128::from(10u8);
        let new_deposit_token_address = String::from("updated");
        let update_config_msg = ExecuteMsg::UpdateConfig {
            threshold: new_threshold.clone(),
            max_voting_period: new_voting_period.clone(),
            proposal_deposit_amount: new_proposal_deposit_amount,
            proposal_deposit_token_address: new_deposit_token_address.clone(),
        };
        let res = app.execute_contract(
            Addr::unchecked(VOTER1),
            dao_addr.clone(),
            &update_config_msg,
            &[],
        );
        assert!(res.is_err());
        let res = app.execute_contract(
            Addr::unchecked(OWNER),
            dao_addr.clone(),
            &update_config_msg,
            &[],
        );
        assert!(res.is_err());

        let wasm_msg = WasmMsg::Execute {
            contract_addr: dao_addr.clone().into(),
            msg: to_binary(&update_config_msg).unwrap(),
            funds: vec![],
        };

        // Update config proposal must be made
        let proposal_msg = ExecuteMsg::Propose {
            title: String::from("Change params"),
            description: String::from("Updates threshold and max voting params"),
            msgs: vec![wasm_msg.into()],
            latest: None,
        };
        let res = app
            .execute_contract(Addr::unchecked(OWNER), dao_addr.clone(), &proposal_msg, &[])
            .unwrap();
        let proposal_id: u64 = res.custom_attrs(1)[2].value.parse().unwrap();

        // Imediately passes on yes vote
        let yes_vote = ExecuteMsg::Vote {
            proposal_id,
            vote: Vote::Yes,
        };
        let res = app.execute_contract(Addr::unchecked(VOTER3), dao_addr.clone(), &yes_vote, &[]);
        assert!(res.is_ok());

        // Execute
        let execution = ExecuteMsg::Execute { proposal_id };
        let res = app.execute_contract(Addr::unchecked(OWNER), dao_addr.clone(), &execution, &[]);
        assert!(res.is_ok());

        // Check that config was updated
        let res: ConfigResponse = app
            .wrap()
            .query_wasm_smart(&dao_addr, &QueryMsg::GetConfig {})
            .unwrap();

        let cw20 = Cw20Contract(cw20_addr);
        assert_eq!(
            res,
            ConfigResponse {
                config: Config {
                    threshold: new_threshold.clone(),
                    max_voting_period: new_voting_period.clone(),
                    cw20_addr: cw20,
                    proposal_deposit: ProposalDeposit {
                        amount: new_proposal_deposit_amount,
                        token_address: Cw20Contract(Addr::unchecked(new_deposit_token_address)),
                    }
                },
            }
        )
    }

    #[test]
    fn test_config_query() {
        let mut app = mock_app();

        let voting_period = Duration::Time(2000000);
        let threshold = Threshold::ThresholdQuorum {
            threshold: Decimal::percent(51),
            quorum: Decimal::percent(10),
        };
        let (dao_addr, cw20_addr) = setup_test_case(
            &mut app,
            threshold.clone(),
            voting_period.clone(),
            coins(100, NATIVE_TOKEN_DENOM),
        );

        let config_query = QueryMsg::GetConfig {};
        let res: ConfigResponse = app
            .wrap()
            .query_wasm_smart(&dao_addr, &config_query)
            .unwrap();

        assert_eq!(
            res,
            ConfigResponse {
                config: Config {
                    threshold,
                    max_voting_period: voting_period,
                    cw20_addr: Cw20Contract(cw20_addr.clone()),
                    proposal_deposit: ProposalDeposit {
                        amount: Uint128::zero(),
                        token_address: Cw20Contract(cw20_addr),
                    }
                },
            }
        )
    }

    #[test]
    fn test_proposal_deposit_works() {
        let mut app = mock_app();

        let voting_period = Duration::Time(2000000);
        let threshold = Threshold::AbsolutePercentage {
            percentage: Decimal::percent(20),
        };
        let (dao_addr, cw20_addr) = setup_test_case(
            &mut app,
            threshold.clone(),
            voting_period,
            coins(10, NATIVE_TOKEN_DENOM),
        );

        let cw20 = Cw20Contract(cw20_addr.clone());

        let initial_owner_cw20_balance = cw20.balance(&app, Addr::unchecked(OWNER)).unwrap();

        // ensure we have cash to cover the proposal
        let contract_bal = app
            .wrap()
            .query_balance(&dao_addr, NATIVE_TOKEN_DENOM)
            .unwrap();
        assert_eq!(contract_bal, coin(10, NATIVE_TOKEN_DENOM));

        let proposal_deposit_amount = Uint128::new(10);

        let update_config_msg = ExecuteMsg::UpdateConfig {
            threshold,
            max_voting_period: voting_period,
            proposal_deposit_amount,
            proposal_deposit_token_address: cw20_addr.to_string(),
        };
        let res = app.execute_contract(dao_addr.clone(), dao_addr.clone(), &update_config_msg, &[]);
        assert!(res.is_ok());

        // Give dao allowance for proposal
        let allowance = Cw20ExecuteMsg::IncreaseAllowance {
            spender: dao_addr.clone().into(),
            amount: proposal_deposit_amount,
            expires: None,
        };
        let res = app.execute_contract(Addr::unchecked(OWNER), cw20_addr.clone(), &allowance, &[]);
        assert!(res.is_ok());

        // create proposal with 0 vote power
        let proposal = pay_somebody_proposal();
        let res = app
            .execute_contract(Addr::unchecked(OWNER), dao_addr.clone(), &proposal, &[])
            .unwrap();

        // Get the proposal id from the logs
        let proposal_id: u64 = res.custom_attrs(1)[2].value.parse().unwrap();

        // Check proposal deposit was made
        let balance = cw20.balance(&app, Addr::unchecked(OWNER)).unwrap();
        let expected_balance = initial_owner_cw20_balance
            .checked_sub(proposal_deposit_amount)
            .unwrap();
        assert_eq!(balance, expected_balance);

        // Only Passed can be executed
        let execution = ExecuteMsg::Execute { proposal_id };
        let err = app
            .execute_contract(Addr::unchecked(OWNER), dao_addr.clone(), &execution, &[])
            .unwrap_err();
        assert_eq!(
            ContractError::WrongExecuteStatus {},
            err.downcast().unwrap()
        );

        // Vote it, so it passes
        let vote = ExecuteMsg::Vote {
            proposal_id,
            vote: Vote::Yes,
        };
        let res = app
            .execute_contract(Addr::unchecked(VOTER3), dao_addr.clone(), &vote, &[])
            .unwrap();
        assert_eq!(
            res.custom_attrs(1),
            [
                ("action", "vote"),
                ("sender", VOTER3),
                ("proposal_id", proposal_id.to_string().as_str()),
                ("status", "Passed"),
            ],
        );

        // Execute works. Anybody can execute Passed proposals
        let res = app
            .execute_contract(Addr::unchecked(SOMEBODY), dao_addr.clone(), &execution, &[])
            .unwrap();
        assert_eq!(
            res.custom_attrs(1),
            [
                ("action", "execute"),
                ("sender", SOMEBODY),
                ("proposal_id", proposal_id.to_string().as_str()),
            ],
        );

        // Check deposit has been refunded
        let balance = cw20.balance(&app, Addr::unchecked(OWNER)).unwrap();
        assert_eq!(balance, initial_owner_cw20_balance);
    }
}
