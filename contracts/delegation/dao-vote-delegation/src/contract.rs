use cosmwasm_std::Order;
#[cfg(not(feature = "library"))]
use cosmwasm_std::{
    entry_point, to_json_binary, Binary, Decimal, Deps, DepsMut, Env, MessageInfo, Response,
    StdError, StdResult, Uint128,
};
use cw2::{get_contract_version, set_contract_version};
use cw_snapshot_vector_map::LoadedItem;
use cw_storage_plus::Bound;
use cw_utils::{maybe_addr, nonpayable};
use dao_hooks::vote::VoteHookMsg;
use dao_interface::voting::InfoResponse;
use semver::Version;

use crate::helpers::{calculate_delegated_vp, get_udvp, get_voting_power, is_delegate_registered};
use crate::msg::{
    DelegateResponse, DelegatesResponse, DelegationsResponse, ExecuteMsg, InstantiateMsg,
    MigrateMsg, OptionalUpdate, QueryMsg,
};
use crate::state::{
    Config, Delegate, Delegation, CONFIG, DAO, DELEGATED_VP, DELEGATED_VP_AMOUNTS, DELEGATES,
    DELEGATIONS, DELEGATION_IDS, PERCENT_DELEGATED, UNVOTED_DELEGATED_VP,
};
use crate::ContractError;

pub(crate) const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
pub(crate) const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const DEFAULT_LIMIT: u32 = 10;
pub const MAX_LIMIT: u32 = 50;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let dao = msg
        .dao
        .map(|d| deps.api.addr_validate(&d))
        .transpose()?
        .unwrap_or(info.sender);

    DAO.save(deps.storage, &dao)?;

    CONFIG.save(
        deps.storage,
        &Config {
            vp_cap_percent: msg.vp_cap_percent,
        },
    )?;

    Ok(Response::new().add_attribute("dao", dao))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Register {} => execute_register(deps, env, info),
        ExecuteMsg::Unregister {} => execute_unregister(deps, env, info),
        ExecuteMsg::Delegate { delegate, percent } => {
            execute_delegate(deps, env, info, delegate, percent)
        }
        ExecuteMsg::Undelegate { delegate } => execute_undelegate(deps, env, info, delegate),
        ExecuteMsg::UpdateConfig { vp_cap_percent, .. } => {
            execute_update_config(deps, info, vp_cap_percent)
        }
        // ExecuteMsg::StakeChangeHook(msg) => execute_stake_changed(deps, env, info, msg),
        // ExecuteMsg::NftStakeChangeHook(msg) => execute_nft_stake_changed(deps, env, info, msg),
        // ExecuteMsg::MemberChangedHook(msg) => execute_membership_changed(deps, env, info, msg),
        ExecuteMsg::VoteHook(vote_hook) => execute_vote_hook(deps, env, info, vote_hook),
    }
}

fn execute_register(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    nonpayable(&info)?;

    let delegate = info.sender;

    if is_delegate_registered(deps.as_ref(), &delegate, None)? {
        return Err(ContractError::DelegateAlreadyRegistered {});
    }

    // ensure delegate has voting power in the DAO
    let vp = get_voting_power(
        deps.as_ref(),
        &delegate,
        // use next block height since voting power takes effect at the start of
        // the next block. if the delegate changed their voting power in the
        // current block, we need to use the new value.
        env.block.height + 1,
    )?;
    if vp.is_zero() {
        return Err(ContractError::NoVotingPower {});
    }

    // ensure delegate has no delegations
    let has_delegations = !DELEGATION_IDS.prefix(&delegate).is_empty(deps.storage);
    if has_delegations {
        return Err(ContractError::UndelegateBeforeRegistering {});
    }

    DELEGATES.save(deps.storage, delegate, &Delegate {}, env.block.height)?;

    Ok(Response::new())
}

fn execute_unregister(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;

    let delegate = info.sender;

    if !is_delegate_registered(deps.as_ref(), &delegate, None)? {
        return Err(ContractError::DelegateNotRegistered {});
    }

    DELEGATES.remove(deps.storage, delegate, env.block.height)?;

    Ok(Response::new())
}

fn execute_delegate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    delegate: String,
    percent: Decimal,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;

    if percent <= Decimal::zero() {
        return Err(ContractError::InvalidVotingPowerPercent {});
    }

    let delegator = info.sender;

    // delegates cannot delegate to others
    if is_delegate_registered(deps.as_ref(), &delegator, None)? {
        return Err(ContractError::DelegatesCannotDelegate {});
    }

    // prevent self delegation
    let delegate = deps.api.addr_validate(&delegate)?;
    if delegate == delegator {
        return Err(ContractError::CannotDelegateToSelf {});
    }

    // ensure delegator has voting power in the DAO
    let vp = get_voting_power(
        deps.as_ref(),
        &delegator,
        // use next block height since voting power takes effect at the start of
        // the next block. if the delegator changed their voting power in the
        // current block, we need to use the new value.
        env.block.height + 1,
    )?;
    if vp.is_zero() {
        return Err(ContractError::NoVotingPower {});
    }

    // prevent duplicate delegation
    let delegation_exists = DELEGATION_IDS.has(deps.storage, (&delegator, &delegate));
    if delegation_exists {
        return Err(ContractError::DelegationAlreadyExists {});
    }

    // ensure delegate is registered
    if !is_delegate_registered(deps.as_ref(), &delegate, None)? {
        return Err(ContractError::DelegateNotRegistered {});
    }

    // ensure not delegating more than 100%
    let current_percent_delegated = PERCENT_DELEGATED
        .may_load(deps.storage, &delegator)?
        .unwrap_or_default();
    let new_percent_delegated = current_percent_delegated.checked_add(percent)?;
    if new_percent_delegated > Decimal::one() {
        return Err(ContractError::CannotDelegateMoreThan100Percent {
            current: current_percent_delegated
                .checked_mul(Decimal::new(100u128.into()))?
                .to_string(),
        });
    }

    // add new delegation
    let delegation_id = DELEGATIONS.push(
        deps.storage,
        &delegator,
        &Delegation {
            delegate: delegate.clone(),
            percent,
        },
        env.block.height,
        // TODO: expiry??
        None,
    )?;

    DELEGATION_IDS.save(deps.storage, (&delegator, &delegate), &delegation_id)?;
    PERCENT_DELEGATED.save(deps.storage, &delegator, &new_percent_delegated)?;

    // add the delegated VP to the delegate's total delegated VP
    let delegated_vp = calculate_delegated_vp(vp, percent);
    DELEGATED_VP.update(
        deps.storage,
        &delegate,
        env.block.height,
        |vp| -> StdResult<Uint128> {
            Ok(vp
                .unwrap_or_default()
                .checked_add(delegated_vp)
                .map_err(StdError::overflow)?)
        },
    )?;
    DELEGATED_VP_AMOUNTS.save(deps.storage, (&delegator, &delegate), &delegated_vp)?;

    Ok(Response::new())
}

fn execute_undelegate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    delegate: String,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;

    let delegator = info.sender;
    let delegate = deps.api.addr_validate(&delegate)?;

    // ensure delegation exists
    let existing_id = DELEGATION_IDS
        .load(deps.storage, (&delegator, &delegate))
        .map_err(|_| ContractError::DelegationDoesNotExist {})?;

    // if delegation exists above, percent will exist
    let current_percent_delegated = PERCENT_DELEGATED.load(deps.storage, &delegator)?;

    // retrieve and remove delegation
    let delegation = DELEGATIONS.remove(deps.storage, &delegator, existing_id, env.block.height)?;
    DELEGATION_IDS.remove(deps.storage, (&delegator, &delegate));

    // update delegator's percent delegated
    let new_percent_delegated = current_percent_delegated.checked_sub(delegation.percent)?;
    PERCENT_DELEGATED.save(deps.storage, &delegator, &new_percent_delegated)?;

    // remove delegated VP from delegate's total delegated VP
    let current_delegated_vp = DELEGATED_VP_AMOUNTS.load(deps.storage, (&delegator, &delegate))?;
    DELEGATED_VP.update(
        deps.storage,
        &delegate,
        env.block.height,
        |vp| -> StdResult<Uint128> {
            Ok(vp
                // must exist if delegation was added in the past
                .ok_or(StdError::not_found("delegate's total delegated VP"))?
                .checked_sub(current_delegated_vp)
                .map_err(StdError::overflow)?)
        },
    )?;
    DELEGATED_VP_AMOUNTS.remove(deps.storage, (&delegator, &delegate));

    Ok(Response::new())
}

fn execute_update_config(
    deps: DepsMut,
    info: MessageInfo,
    vp_cap_percent: Option<OptionalUpdate<Decimal>>,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;

    // only the DAO can update the config
    let dao = DAO.load(deps.storage)?;
    if info.sender != dao {
        return Err(ContractError::Unauthorized {});
    }

    let mut config = CONFIG.load(deps.storage)?;

    if let Some(vp_cap_percent) = vp_cap_percent {
        match vp_cap_percent {
            OptionalUpdate::Set(vp_cap_percent) => config.vp_cap_percent = Some(vp_cap_percent),
            OptionalUpdate::Clear => config.vp_cap_percent = None,
        }
    }

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new())
}

pub fn execute_vote_hook(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    vote_hook: VoteHookMsg,
) -> Result<Response, ContractError> {
    let proposal_module = info.sender;

    // TODO: validate proposal module

    match vote_hook {
        VoteHookMsg::NewVote {
            proposal_id,
            voter,
            power,
            height,
            is_first_vote,
            ..
        } => {
            // if first vote, update the unvoted delegated VP for their
            // delegates by subtracting. if not first vote, this has already
            // been done.
            if is_first_vote {
                let delegator = deps.api.addr_validate(&voter)?;
                let delegates = DELEGATIONS.load_all(deps.storage, &delegator, env.block.height)?;
                for LoadedItem {
                    item: Delegation { delegate, percent },
                    ..
                } in delegates
                {
                    let udvp = get_udvp(
                        deps.as_ref(),
                        &delegate,
                        &proposal_module,
                        proposal_id,
                        height,
                    )?;

                    let delegated_vp = calculate_delegated_vp(power, percent);

                    // remove the delegator's delegated VP from the delegate's
                    // unvoted delegated VP for this proposal since this
                    // delegator just voted.
                    let new_udvp = udvp.checked_sub(delegated_vp)?;

                    UNVOTED_DELEGATED_VP.save(
                        deps.storage,
                        (&delegate, &proposal_module, proposal_id),
                        &new_udvp,
                    )?;
                }
            }
        }
    }

    Ok(Response::new().add_attribute("action", "vote_hook"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Info {} => Ok(to_json_binary(&query_info(deps)?)?),
        QueryMsg::Delegates { start_after, limit } => {
            Ok(to_json_binary(&query_delegates(deps, start_after, limit)?)?)
        }
        QueryMsg::Delegations {
            delegator,
            height,
            offset,
            limit,
        } => Ok(to_json_binary(&query_delegations(
            deps, env, delegator, height, offset, limit,
        )?)?),
        QueryMsg::UnvotedDelegatedVotingPower {
            delegate,
            proposal_module,
            proposal_id,
            height,
        } => Ok(to_json_binary(&query_unvoted_delegated_vp(
            deps,
            delegate,
            proposal_module,
            proposal_id,
            height,
        )?)?),
    }
}

fn query_info(deps: Deps) -> StdResult<InfoResponse> {
    let info = get_contract_version(deps.storage)?;
    Ok(InfoResponse { info })
}

fn query_delegates(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<DelegatesResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

    let start = maybe_addr(deps.api, start_after)?.map(Bound::exclusive);

    let delegates = DELEGATES
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|delegate| {
            delegate.map(|(delegate, _)| -> StdResult<DelegateResponse> {
                let power = DELEGATED_VP
                    .may_load(deps.storage, &delegate)?
                    .unwrap_or_default();
                Ok(DelegateResponse { delegate, power })
            })?
        })
        .collect::<StdResult<_>>()?;

    Ok(DelegatesResponse { delegates })
}

fn query_delegations(
    deps: Deps,
    env: Env,
    delegator: String,
    height: Option<u64>,
    offset: Option<u64>,
    limit: Option<u64>,
) -> StdResult<DelegationsResponse> {
    let height = height.unwrap_or(env.block.height);
    let delegator = deps.api.addr_validate(&delegator)?;
    let delegations = DELEGATIONS
        .load(deps.storage, &delegator, height, limit, offset)?
        .into_iter()
        .map(|d| d.item)
        .collect();
    Ok(DelegationsResponse {
        delegations,
        height,
    })
}

fn query_unvoted_delegated_vp(
    deps: Deps,
    delegate: String,
    proposal_module: String,
    proposal_id: u64,
    height: u64,
) -> StdResult<Uint128> {
    let delegate = deps.api.addr_validate(&delegate)?;

    // if delegate not registered, they have no unvoted delegated VP.
    if !is_delegate_registered(deps, &delegate, Some(height))? {
        return Ok(Uint128::zero());
    }

    let proposal_module = deps.api.addr_validate(&proposal_module)?;

    get_udvp(deps, &delegate, &proposal_module, proposal_id, height)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    let contract_version = get_contract_version(deps.storage)?;

    if contract_version.contract != CONTRACT_NAME {
        return Err(ContractError::MigrationErrorIncorrectContract {
            expected: CONTRACT_NAME.to_string(),
            actual: contract_version.contract,
        });
    }

    let new_version: Version = CONTRACT_VERSION.parse()?;
    let current_version: Version = contract_version.version.parse()?;

    // only allow upgrades
    if new_version <= current_version {
        return Err(ContractError::MigrationErrorInvalidVersion {
            new: new_version.to_string(),
            current: current_version.to_string(),
        });
    }

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::default())
}
