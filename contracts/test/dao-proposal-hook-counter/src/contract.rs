#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Uint128,
};
use cw2::set_contract_version;
use dao_hooks::stake::StakeChangedHookMsg;
use dao_hooks::{proposal::ProposalHookMsg, vote::VoteHookMsg};

use crate::error::ContractError;
use crate::msg::{CountResponse, ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{
    Config, CONFIG, PROPOSAL_COUNTER, STAKE_COUNTER, STATUS_CHANGED_COUNTER, VOTE_COUNTER,
};

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
    STAKE_COUNTER.save(deps.storage, &Uint128::zero())?;
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
        ExecuteMsg::StakeChangeHook(stake_hook) => execute_stake_hook(deps, env, info, stake_hook),
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
            count = count.checked_add(1).unwrap_or_default();
            PROPOSAL_COUNTER.save(deps.storage, &count)?;
        }
        ProposalHookMsg::ProposalStatusChanged { .. } => {
            let mut count = STATUS_CHANGED_COUNTER.load(deps.storage)?;
            count = count.checked_add(1).unwrap_or_default();
            STATUS_CHANGED_COUNTER.save(deps.storage, &count)?;
        }
    }

    Ok(Response::new().add_attribute("action", "proposal_hook"))
}

pub fn execute_stake_hook(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    stake_hook: StakeChangedHookMsg,
) -> Result<Response, ContractError> {
    match stake_hook {
        StakeChangedHookMsg::Stake { .. } => {
            let mut count = STAKE_COUNTER.load(deps.storage)?;
            count = count.checked_add(Uint128::new(1))?;
            STAKE_COUNTER.save(deps.storage, &count)?;
        }
        StakeChangedHookMsg::Unstake { .. } => {
            let mut count = STAKE_COUNTER.load(deps.storage)?;
            count = count.checked_add(Uint128::new(1))?;
            STAKE_COUNTER.save(deps.storage, &count)?;
        }
    }

    Ok(Response::new().add_attribute("action", "stake_hook"))
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
            count = count.checked_add(1).unwrap_or_default();
            VOTE_COUNTER.save(deps.storage, &count)?;
        }
    }

    Ok(Response::new().add_attribute("action", "vote_hook"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::ProposalCounter {} => to_json_binary(&CountResponse {
            count: PROPOSAL_COUNTER.load(deps.storage)?,
        }),
        QueryMsg::StakeCounter {} => to_json_binary(&STAKE_COUNTER.load(deps.storage)?),
        QueryMsg::StatusChangedCounter {} => to_json_binary(&CountResponse {
            count: STATUS_CHANGED_COUNTER.load(deps.storage)?,
        }),
        QueryMsg::VoteCounter {} => to_json_binary(&CountResponse {
            count: VOTE_COUNTER.load(deps.storage)?,
        }),
    }
}
