#[cfg(not(feature = "library"))]
use cosmwasm_std::{
    entry_point, to_json_binary, Binary, Decimal, Deps, DepsMut, Env, MessageInfo, Response,
    StdError, StdResult, Uint128,
};
use cosmwasm_std::{Addr, Order};
use cw2::{get_contract_version, set_contract_version};
use cw_paginate_storage::paginate_map_keys;
use cw_storage_plus::Bound;
use cw_utils::{maybe_addr, nonpayable};
use dao_interface::helpers::OptionalUpdate;
use dao_interface::state::{ProposalModule, ProposalModuleStatus};
use dao_interface::voting::InfoResponse;
use dao_voting::delegation::calculate_delegated_vp;
use semver::Version;

use crate::helpers::{
    ensure_setup, get_udvp, get_voting_power, is_delegate_registered, unregister_delegate,
};
use crate::hooks::{
    execute_membership_changed, execute_nft_stake_changed, execute_stake_changed, execute_vote_hook,
};
use crate::msg::{
    DelegateResponse, DelegatesResponse, DelegationsResponse, ExecuteMsg, InstantiateMsg,
    MigrateMsg, QueryMsg,
};
use crate::state::{
    Config, Delegate, Delegation, CONFIG, DAO, DELEGATED_VP, DELEGATES, DELEGATIONS,
    DELEGATION_IDS, PERCENT_DELEGATED, PROPOSAL_HOOK_CALLERS, VOTING_POWER_HOOK_CALLERS,
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

    // sync proposal modules with no limit if not disabled. this should succeed
    // for most DAOs as the query will not run out of gas with only a few
    // proposal modules.
    if !msg.no_sync_proposal_modules.unwrap_or(false) {
        execute_sync_proposal_modules(deps, None, None)?;
    }

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
        ExecuteMsg::UpdateVotingPowerHookCallers { add, remove } => {
            execute_update_voting_power_hook_callers(deps, info, add, remove)
        }
        ExecuteMsg::SyncProposalModules { start_after, limit } => {
            execute_sync_proposal_modules(deps, start_after, limit)
        }
        ExecuteMsg::UpdateConfig { vp_cap_percent, .. } => {
            execute_update_config(deps, info, vp_cap_percent)
        }
        ExecuteMsg::StakeChangeHook(msg) => execute_stake_changed(deps, env, info, msg),
        ExecuteMsg::NftStakeChangeHook(msg) => execute_nft_stake_changed(deps, env, info, msg),
        ExecuteMsg::MemberChangedHook(msg) => execute_membership_changed(deps, env, info, msg),
        ExecuteMsg::VoteHook(vote_hook) => execute_vote_hook(deps, env, info, vote_hook),
    }
}

fn execute_register(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    ensure_setup(deps.as_ref())?;

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
    ensure_setup(deps.as_ref())?;

    let delegate = info.sender;

    if !is_delegate_registered(deps.as_ref(), &delegate, None)? {
        return Err(ContractError::DelegateNotRegistered {});
    }

    unregister_delegate(deps, &delegate, env.block.height)?;

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
    ensure_setup(deps.as_ref())?;

    if percent <= Decimal::zero() || percent > Decimal::one() {
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

    // ensure delegate is registered
    if !is_delegate_registered(deps.as_ref(), &delegate, None)? {
        return Err(ContractError::DelegateNotRegistered {});
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

    let current_percent_delegated = PERCENT_DELEGATED
        .may_load(deps.storage, &delegator)?
        .unwrap_or_default();

    let existing_delegation_id = DELEGATION_IDS.may_load(deps.storage, (&delegator, &delegate))?;

    // will be set below, differing based on whether this is a new delegation or
    // an update to an existing one
    let new_total_percent_delegated: Decimal;
    let current_delegated_vp: Uint128;

    // update an existing delegation
    if let Some(existing_delegation_id) = existing_delegation_id {
        let mut existing_delegation =
            DELEGATIONS.load_item(deps.storage, &delegator, existing_delegation_id)?;

        // remove existing percent and replace with new percent
        new_total_percent_delegated = current_percent_delegated
            .checked_sub(existing_delegation.percent)?
            .checked_add(percent)?;

        // compute current delegated VP to replace based on existing percent
        // before it's replaced
        current_delegated_vp = calculate_delegated_vp(vp, existing_delegation.percent);

        // replace delegation with updated percent
        DELEGATIONS.remove(
            deps.storage,
            &delegator,
            existing_delegation_id,
            env.block.height,
        )?;
        existing_delegation.percent = percent;
        let new_delegation_id = DELEGATIONS.push(
            deps.storage,
            &delegator,
            &existing_delegation,
            env.block.height,
            // TODO: expiry??
            None,
        )?;
        DELEGATION_IDS.save(deps.storage, (&delegator, &delegate), &new_delegation_id)?;
    }
    // create a new delegation
    else {
        new_total_percent_delegated = current_percent_delegated.checked_add(percent)?;
        current_delegated_vp = Uint128::zero();

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
    }

    // ensure not delegating more than 100%
    if new_total_percent_delegated > Decimal::one() {
        return Err(ContractError::CannotDelegateMoreThan100Percent {
            current: current_percent_delegated
                .checked_mul(Decimal::new(100u128.into()))?
                .to_string(),
            attempt: new_total_percent_delegated
                .checked_mul(Decimal::new(100u128.into()))?
                .to_string(),
        });
    }

    PERCENT_DELEGATED.save(deps.storage, &delegator, &new_total_percent_delegated)?;

    // calculate the new delegated VP and add to the delegate's total
    let new_delegated_vp = calculate_delegated_vp(vp, percent);
    // this `update` function loads the latest delegated VP, even if it was
    // updated before in this block, and then saves the new total at the current
    // block, which will be reflected in historical queries starting from the
    // NEXT block. if future delegations/undelegations/voting power changes
    // occur in this block, they will immediately load the latest state, and
    // update the total that will be reflected in historical queries starting
    // from the next block.
    DELEGATED_VP.update(
        deps.storage,
        &delegate,
        env.block.height,
        |vp| -> StdResult<Uint128> {
            vp.unwrap_or_default()
                // remove the current delegated VP from the delegate's total and
                // replace it with the new delegated VP. if this is a new
                // delegation, this will be zero.
                .checked_sub(current_delegated_vp)
                .map_err(StdError::overflow)?
                .checked_add(new_delegated_vp)
                .map_err(StdError::overflow)
        },
    )?;

    Ok(Response::new())
}

fn execute_undelegate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    delegate: String,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    ensure_setup(deps.as_ref())?;

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

    let vp = get_voting_power(
        deps.as_ref(),
        &delegator,
        // use next block height since voting power takes effect at the start of
        // the next block. if the delegator changed their voting power in the
        // current block, we need to use the new value.
        env.block.height + 1,
    )?;

    // remove delegated VP from delegate's total delegated VP
    let current_delegated_vp = calculate_delegated_vp(vp, delegation.percent);
    // this `update` function loads the latest delegated VP, even if it was
    // updated before in this block, and then saves the new total at the current
    // block, which will be reflected in historical queries starting from the
    // NEXT block. if future delegations/undelegations/voting power changes
    // occur in this block, they will immediately load the latest state, and
    // update the total that will be reflected in historical queries starting
    // from the next block.
    DELEGATED_VP.update(
        deps.storage,
        &delegate,
        env.block.height,
        |vp| -> StdResult<Uint128> {
            vp
                // must exist if delegation was added in the past
                .ok_or(StdError::not_found("delegate's total delegated VP"))?
                .checked_sub(current_delegated_vp)
                .map_err(StdError::overflow)
        },
    )?;

    Ok(Response::new())
}

fn execute_update_voting_power_hook_callers(
    deps: DepsMut,
    info: MessageInfo,
    add: Option<Vec<String>>,
    remove: Option<Vec<String>>,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;

    // only the DAO can update the voting power hook callers
    let dao = DAO.load(deps.storage)?;
    if info.sender != dao {
        return Err(ContractError::Unauthorized {});
    }

    if let Some(add) = add {
        for addr in add {
            VOTING_POWER_HOOK_CALLERS.save(deps.storage, deps.api.addr_validate(&addr)?, &())?;
        }
    }

    if let Some(remove) = remove {
        for addr in remove {
            VOTING_POWER_HOOK_CALLERS.remove(deps.storage, deps.api.addr_validate(&addr)?);
        }
    }

    Ok(Response::new().add_attribute("action", "update_voting_power_hook_callers"))
}

fn execute_sync_proposal_modules(
    deps: DepsMut,
    start_after: Option<String>,
    limit: Option<u32>,
) -> Result<Response, ContractError> {
    let dao = DAO.load(deps.storage)?;
    let proposal_modules: Vec<ProposalModule> = deps.querier.query_wasm_smart(
        dao,
        &dao_interface::msg::QueryMsg::ProposalModules { start_after, limit },
    )?;

    let mut enabled = 0;
    let mut disabled = 0;
    for proposal_module in proposal_modules {
        if proposal_module.status == ProposalModuleStatus::Enabled {
            enabled += 1;
            PROPOSAL_HOOK_CALLERS.save(deps.storage, proposal_module.address, &())?;
        } else {
            disabled += 1;
            PROPOSAL_HOOK_CALLERS.remove(deps.storage, proposal_module.address);
        }
    }

    Ok(Response::new()
        .add_attribute("action", "sync_proposal_modules")
        .add_attribute("enabled", enabled.to_string())
        .add_attribute("disabled", disabled.to_string()))
}

fn execute_update_config(
    deps: DepsMut,
    info: MessageInfo,
    vp_cap_percent: OptionalUpdate<Decimal>,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;

    // only the DAO can update the config
    let dao = DAO.load(deps.storage)?;
    if info.sender != dao {
        return Err(ContractError::Unauthorized {});
    }

    let mut config = CONFIG.load(deps.storage)?;

    vp_cap_percent.maybe_update(|value| {
        config.vp_cap_percent = value;
    });

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new())
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
        QueryMsg::ProposalModules { start_after, limit } => Ok(to_json_binary(
            &query_proposal_modules(deps, start_after, limit)?,
        )?),
        QueryMsg::VotingPowerHookCallers { start_after, limit } => Ok(to_json_binary(
            &query_voting_power_hook_callers(deps, start_after, limit)?,
        )?),
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

fn query_proposal_modules(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Vec<Addr>> {
    paginate_map_keys(
        deps,
        &PROPOSAL_HOOK_CALLERS,
        start_after
            .map(|s| deps.api.addr_validate(&s))
            .transpose()?,
        limit,
        Order::Ascending,
    )
}

fn query_voting_power_hook_callers(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Vec<Addr>> {
    paginate_map_keys(
        deps,
        &VOTING_POWER_HOOK_CALLERS,
        start_after
            .map(|s| deps.api.addr_validate(&s))
            .transpose()?,
        limit,
        Order::Ascending,
    )
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
