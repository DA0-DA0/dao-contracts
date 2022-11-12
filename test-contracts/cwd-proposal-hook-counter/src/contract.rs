#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw2::set_contract_version;
use cwd_proposal_hooks::ProposalHookMsg;
use cwd_vote_hooks::VoteHookMsg;

use crate::error::ContractError;
use crate::msg::{CountResponse, ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{Config, CONFIG, PROPOSAL_COUNTER, STATUS_CHANGED_COUNTER, VOTE_COUNTER};

const CONTRACT_NAME: &str = "crates.io:proposal-hooks-counter";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let config = Config {
        should_error: msg.should_error,
    };
    CONFIG.save(deps.storage, &config)?;
    PROPOSAL_COUNTER.save(deps.storage, &0)?;
    VOTE_COUNTER.save(deps.storage, &0)?;
    STATUS_CHANGED_COUNTER.save(deps.storage, &0)?;
    Ok(Response::new().add_attribute("action", "instantiate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if config.should_error {
        return Err(ContractError::Unauthorized {});
    }

    match msg {
        ExecuteMsg::ProposalHook(proposal_hook) => {
            execute_proposal_hook(deps, env, info, proposal_hook)
        }
        ExecuteMsg::VoteHook(vote_hook) => execute_vote_hook(deps, env, info, vote_hook),
    }
}

pub fn execute_proposal_hook(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    proposal_hook: ProposalHookMsg,
) -> Result<Response, ContractError> {
    match proposal_hook {
        ProposalHookMsg::NewProposal { .. } => {
            let mut count = PROPOSAL_COUNTER.load(deps.storage)?;
            count += 1;
            PROPOSAL_COUNTER.save(deps.storage, &count)?;
        }
        ProposalHookMsg::ProposalStatusChanged { .. } => {
            let mut count = STATUS_CHANGED_COUNTER.load(deps.storage)?;
            count += 1;
            STATUS_CHANGED_COUNTER.save(deps.storage, &count)?;
        }
    }

    Ok(Response::new().add_attribute("action", "proposal_hook"))
}

pub fn execute_vote_hook(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    vote_hook: VoteHookMsg,
) -> Result<Response, ContractError> {
    match vote_hook {
        VoteHookMsg::NewVote { .. } => {
            let mut count = VOTE_COUNTER.load(deps.storage)?;
            count += 1;
            VOTE_COUNTER.save(deps.storage, &count)?;
        }
    }

    Ok(Response::new().add_attribute("action", "vote_hook"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::VoteCounter {} => to_binary(&CountResponse {
            count: VOTE_COUNTER.load(deps.storage)?,
        }),
        QueryMsg::ProposalCounter {} => to_binary(&CountResponse {
            count: PROPOSAL_COUNTER.load(deps.storage)?,
        }),
        QueryMsg::StatusChangedCounter {} => to_binary(&CountResponse {
            count: STATUS_CHANGED_COUNTER.load(deps.storage)?,
        }),
    }
}
