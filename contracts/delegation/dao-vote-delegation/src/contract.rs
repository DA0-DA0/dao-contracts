use cosmwasm_std::{ensure, Addr, Order, Uint128};
#[cfg(not(feature = "library"))]
use cosmwasm_std::{
    entry_point, to_json_binary, Binary, Decimal, Deps, DepsMut, Env, MessageInfo, Response,
    StdResult,
};
use cw2::{get_contract_version, set_contract_version};
use cw_paginate_storage::paginate_map_keys;
use cw_storage_plus::Bound;
use cw_utils::{maybe_addr, nonpayable};
use dao_interface::helpers::OptionalUpdate;
use dao_interface::state::{ProposalModule, ProposalModuleStatus};
use dao_interface::voting::InfoResponse;
use dao_voting::delegation::{
    calculate_delegated_vp, DelegationResponse, RegistrationResponse,
    UnvotedDelegatedVotingPowerResponse, VotingPowerCapResponse,
};
use dao_voting::voting;
use semver::Version;

use crate::helpers::{
    add_delegated_vp, ensure_setup, get_udvp, get_voting_power, handle_new_delegation,
    handle_redelegation, is_delegate_registered, remove_delegated_vp_if_not_expired,
    unregister_delegate, validate_and_update_percent_delegated, validate_delegation,
};
use crate::hooks::{
    execute_membership_changed, execute_nft_stake_changed, execute_stake_changed, execute_vote_hook,
};
use crate::msg::{
    DelegateResponse, DelegatesResponse, DelegationsResponse, ExecuteMsg, InstantiateMsg,
    MigrateMsg, QueryMsg,
};
use crate::state::{
    Config, Delegate, CONFIG, DAO, DELEGATED_VP, DELEGATES, DELEGATIONS, DELEGATION_ENTRIES,
    PERCENT_DELEGATED, PROPOSAL_HOOK_CALLERS, VOTING_POWER_HOOK_CALLERS, VP_CAP_PERCENT,
};
use crate::ContractError;

pub(crate) const CONTRACT_NAME: &str = "crates.io:dao-vote-delegation";
pub(crate) const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const DEFAULT_LIMIT: u32 = 10;

/// in tests on Neutron, with a block max gas of 30M (which is one of the lowest
/// gas limits on any chain), we found that 50 delegations is a safe upper
/// bound, so this defaults to 50.
pub const DEFAULT_MAX_DELEGATIONS: u64 = 50;

const MIN_DELEGATION_VALIDITY_BLOCKS: u64 = 2;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    nonpayable(&info)?;

    let dao = msg
        .dao
        .map(|d| deps.api.addr_validate(&d))
        .transpose()?
        .unwrap_or(info.sender);
    DAO.save(deps.storage, &dao)?;

    if let Some(vp_cap_percent) = msg.vp_cap_percent {
        if vp_cap_percent <= Decimal::zero() || vp_cap_percent > Decimal::one() {
            return Err(ContractError::InvalidVotingPowerPercent {});
        }
    }

    if let Some(delegation_validity_blocks) = msg.delegation_validity_blocks {
        if delegation_validity_blocks < MIN_DELEGATION_VALIDITY_BLOCKS {
            return Err(ContractError::InvalidDelegationValidityBlocks {
                provided: delegation_validity_blocks,
                min: MIN_DELEGATION_VALIDITY_BLOCKS,
            });
        }
    }

    CONFIG.save(
        deps.storage,
        &Config {
            delegation_validity_blocks: msg.delegation_validity_blocks,
            max_delegations: msg.max_delegations.unwrap_or(DEFAULT_MAX_DELEGATIONS),
        },
    )?;
    VP_CAP_PERCENT.save(deps.storage, &msg.vp_cap_percent, env.block.height)?;

    // initialize voting power changed hook callers
    if let Some(vp_hook_callers) = msg.vp_hook_callers {
        for caller in vp_hook_callers {
            VOTING_POWER_HOOK_CALLERS.save(deps.storage, deps.api.addr_validate(&caller)?, &())?;
        }
    }

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
    nonpayable(&info)?;

    match msg {
        ExecuteMsg::Register {} => execute_register(deps, env, info.sender),
        ExecuteMsg::Unregister {} => execute_unregister(deps, env, info.sender),
        ExecuteMsg::Delegate { delegate, percent } => {
            execute_delegate(deps, env, info.sender, delegate, percent)
        }
        ExecuteMsg::Undelegate { delegate } => execute_undelegate(deps, env, info.sender, delegate),
        ExecuteMsg::UpdateVotingPowerHookCallers { add, remove } => {
            execute_update_voting_power_hook_callers(deps, info.sender, add, remove)
        }
        ExecuteMsg::SyncProposalModules { start_after, limit } => {
            execute_sync_proposal_modules(deps, start_after, limit)
        }
        ExecuteMsg::UpdateConfig {
            vp_cap_percent,
            delegation_validity_blocks,
            max_delegations,
        } => execute_update_config(
            deps,
            env,
            info.sender,
            vp_cap_percent,
            delegation_validity_blocks,
            max_delegations,
        ),
        ExecuteMsg::StakeChangeHook(msg) => execute_stake_changed(deps, env, info.sender, msg),
        ExecuteMsg::NftStakeChangeHook(msg) => {
            execute_nft_stake_changed(deps, env, info.sender, msg)
        }
        ExecuteMsg::MemberChangedHook(msg) => {
            execute_membership_changed(deps, env, info.sender, msg)
        }
        ExecuteMsg::VoteHook(vote_hook) => execute_vote_hook(deps, info.sender, vote_hook),
    }
}

fn execute_register(deps: DepsMut, env: Env, delegate: Addr) -> Result<Response, ContractError> {
    ensure_setup(deps.as_ref())?;

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
    ensure!(
        DELEGATION_ENTRIES.prefix(&delegate).is_empty(deps.storage),
        ContractError::CannotRegisterWithDelegations {}
    );

    DELEGATES.save(deps.storage, delegate, &Delegate {}, env.block.height)?;

    Ok(Response::new())
}

fn execute_unregister(deps: DepsMut, env: Env, delegate: Addr) -> Result<Response, ContractError> {
    ensure_setup(deps.as_ref())?;

    if !is_delegate_registered(deps.as_ref(), &delegate, None)? {
        return Err(ContractError::DelegateNotRegistered {});
    }

    unregister_delegate(deps, &delegate, env.block.height)?;

    Ok(Response::new())
}

fn execute_delegate(
    mut deps: DepsMut,
    env: Env,
    delegator: Addr,
    delegate: String,
    new_percent: Decimal,
) -> Result<Response, ContractError> {
    // validations
    ensure_setup(deps.as_ref())?;
    let delegate = deps.api.addr_validate(&delegate)?;
    let vp = validate_delegation(deps.as_ref(), &env, &delegator, &delegate, new_percent)?;

    // load state
    let config = CONFIG.load(deps.storage)?;
    let current_percent_delegated = PERCENT_DELEGATED
        .may_load(deps.storage, &delegator)?
        .unwrap_or_default();
    let existing_delegation_entry =
        DELEGATION_ENTRIES.may_load(deps.storage, (&delegator, &delegate))?;

    // update an existing delegation, returning the new total percent delegated,
    // the new entry, and whether or not delegated VP changed (if not, just the
    // expiration was updated).
    let (new_total_percent, new_entry, delegated_vp_changed) =
        if let Some(existing_delegation_entry) = existing_delegation_entry {
            handle_redelegation(
                deps.branch(),
                &env,
                &delegator,
                &delegate,
                new_percent,
                &config,
                current_percent_delegated,
                vp,
                existing_delegation_entry,
            )?
        }
        // create a new delegation
        else {
            handle_new_delegation(
                deps.branch(),
                &env,
                &delegator,
                &delegate,
                new_percent,
                &config,
                current_percent_delegated,
            )?
        };

    // update the delegation entry (ID and expiration)
    DELEGATION_ENTRIES.save(deps.storage, (&delegator, &delegate), &new_entry)?;

    let new_delegated_vp = calculate_delegated_vp(vp, new_percent);

    // if total percent changed, update delegator and delegate values
    if delegated_vp_changed {
        validate_and_update_percent_delegated(
            deps.branch(),
            &delegator,
            current_percent_delegated,
            new_total_percent,
        )?;

        // add new delegated VP to the delegate's total
        add_delegated_vp(
            deps.storage,
            &env,
            &delegate,
            new_delegated_vp,
            config
                .delegation_validity_blocks
                .map(|expire_in| env.block.height + expire_in),
        )?;
    }

    Ok(Response::new()
        .add_attribute("action", "delegate")
        .add_attribute("delegator", delegator.to_string())
        .add_attribute("delegate", delegate.to_string())
        .add_attribute("percent", new_percent.to_string())
        .add_attribute("vp", new_delegated_vp.to_string()))
}

fn execute_undelegate(
    deps: DepsMut,
    env: Env,
    delegator: Addr,
    delegate: String,
) -> Result<Response, ContractError> {
    ensure_setup(deps.as_ref())?;

    let delegate = deps.api.addr_validate(&delegate)?;

    // ensure delegation exists
    let (existing_id, existing_expiration) = DELEGATION_ENTRIES
        .load(deps.storage, (&delegator, &delegate))
        .map_err(|_| ContractError::DelegationDoesNotExist {})?;

    // retrieve and remove delegation
    let (delegation, _) =
        DELEGATIONS.remove(deps.storage, &delegator, existing_id, env.block.height)?;
    DELEGATION_ENTRIES.remove(deps.storage, (&delegator, &delegate));

    // update the total percent delegated by the delegator
    PERCENT_DELEGATED.update(
        deps.storage,
        &delegator,
        |current_percent| -> StdResult<_> {
            // if delegation above exists, percent will exist. if for some
            // reason it doesn't, it will surface in the checked_sub call below
            // due to an underflow, in which case something is horribly wrong so
            // we should error.
            Ok(current_percent
                .unwrap_or_default()
                .checked_sub(delegation.percent)?)
        },
    )?;

    let vp = get_voting_power(
        deps.as_ref(),
        &delegator,
        // use next block height since voting power takes effect at the start of
        // the next block. if the delegator changed their voting power in the
        // current block, we need to use the new value.
        env.block.height + 1,
    )?;

    // remove delegated VP from delegate's total delegated VP at the current
    // height if the delegation is not expired.
    let delegated_vp = calculate_delegated_vp(vp, delegation.percent);
    remove_delegated_vp_if_not_expired(
        deps.storage,
        &env,
        &delegate,
        delegated_vp,
        existing_expiration,
    )?;

    Ok(Response::new()
        .add_attribute("action", "undelegate")
        .add_attribute("delegator", delegator.to_string())
        .add_attribute("delegate", delegate.to_string())
        .add_attribute("percent", delegation.percent.to_string())
        .add_attribute("vp", delegated_vp.to_string()))
}

fn execute_update_voting_power_hook_callers(
    deps: DepsMut,
    sender: Addr,
    add: Option<Vec<String>>,
    remove: Option<Vec<String>>,
) -> Result<Response, ContractError> {
    // only the DAO can update the voting power hook callers
    let dao = DAO.load(deps.storage)?;
    if sender != dao {
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
    env: Env,
    sender: Addr,
    vp_cap_percent: OptionalUpdate<Decimal>,
    delegation_validity_blocks: OptionalUpdate<u64>,
    max_delegations: Option<u64>,
) -> Result<Response, ContractError> {
    // only the DAO can update the config
    let dao = DAO.load(deps.storage)?;
    if sender != dao {
        return Err(ContractError::Unauthorized {});
    }

    vp_cap_percent.maybe_update_result(|value| -> Result<_, ContractError> {
        if let Some(value) = value {
            if value <= Decimal::zero() || value > Decimal::one() {
                return Err(ContractError::InvalidVotingPowerPercent {});
            }
        }

        VP_CAP_PERCENT.save(deps.storage, &value, env.block.height)?;

        Ok(())
    })?;

    CONFIG.update(deps.storage, |mut config| -> Result<_, ContractError> {
        // updating delegation validity blocks will only apply to new delegations.
        // all existing delegations will keep their existing expiration until it expires.
        delegation_validity_blocks.maybe_update_result(|value| {
            // validate if defined
            if let Some(value) = value {
                if value < MIN_DELEGATION_VALIDITY_BLOCKS {
                    return Err(ContractError::InvalidDelegationValidityBlocks {
                        provided: value,
                        min: MIN_DELEGATION_VALIDITY_BLOCKS,
                    });
                }
            }

            config.delegation_validity_blocks = value;

            Ok(())
        })?;

        if let Some(value) = max_delegations {
            config.max_delegations = value;
        }

        Ok(config)
    })?;

    Ok(Response::new().add_attribute("action", "update_config"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Info {} => Ok(to_json_binary(&query_info(deps)?)?),
        QueryMsg::Registration { delegate, height } => Ok(to_json_binary(&query_registration(
            deps, env, delegate, height,
        )?)?),
        QueryMsg::Delegates { start_after, limit } => Ok(to_json_binary(&query_delegates(
            deps,
            env,
            start_after,
            limit,
        )?)?),
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
            proposal_height,
        } => Ok(to_json_binary(&query_unvoted_delegated_vp(
            deps,
            delegate,
            proposal_module,
            proposal_id,
            proposal_height,
        )?)?),
        QueryMsg::EffectiveUnvotedDelegatedVotingPowerReduction {
            proposal_module,
            proposal_id,
            proposal_height,
            delegate,
            delegated_vp,
        } => Ok(to_json_binary(
            &query_effective_unvoted_delegated_vote_power_reduction(
                deps,
                proposal_module,
                proposal_id,
                proposal_height,
                delegate,
                delegated_vp,
            )?,
        )?),
        QueryMsg::ProposalModules { start_after, limit } => Ok(to_json_binary(
            &query_proposal_modules(deps, start_after, limit)?,
        )?),
        QueryMsg::VotingPowerHookCallers { start_after, limit } => Ok(to_json_binary(
            &query_voting_power_hook_callers(deps, start_after, limit)?,
        )?),
        QueryMsg::Config {} => Ok(to_json_binary(&query_config(deps)?)?),
        QueryMsg::VotingPowerCap { height } => {
            Ok(to_json_binary(&query_voting_power_cap(deps, env, height)?)?)
        }
    }
}

fn query_info(deps: Deps) -> StdResult<InfoResponse> {
    let info = get_contract_version(deps.storage)?;
    Ok(InfoResponse { info })
}

fn query_registration(
    deps: Deps,
    env: Env,
    delegate: String,
    height: Option<u64>,
) -> StdResult<RegistrationResponse> {
    let height = height.unwrap_or(env.block.height);
    let delegate = deps.api.addr_validate(&delegate)?;

    let registered = is_delegate_registered(deps, &delegate, Some(height))?;
    let power = DELEGATED_VP
        .load(deps.storage, delegate, height)?
        .unwrap_or_default();

    Ok(RegistrationResponse {
        registered,
        power,
        height,
    })
}

fn query_delegates(
    deps: Deps,
    env: Env,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<DelegatesResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT) as usize;

    let start = maybe_addr(deps.api, start_after)?.map(Bound::exclusive);

    let delegates = DELEGATES
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|delegate| {
            delegate.map(|(delegate, _)| -> StdResult<DelegateResponse> {
                let power = DELEGATED_VP
                    .load(deps.storage, delegate.clone(), env.block.height)?
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
        .map(|d| -> StdResult<DelegationResponse> {
            let active = is_delegate_registered(deps, &d.item.delegate, Some(height))?;
            Ok(DelegationResponse {
                delegate: d.item.delegate,
                percent: d.item.percent,
                active,
                expiration_height: d.expiration,
            })
        })
        .collect::<StdResult<_>>()?;
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
) -> StdResult<UnvotedDelegatedVotingPowerResponse> {
    let delegate = deps.api.addr_validate(&delegate)?;

    // if delegate not registered, they have no unvoted delegated VP.
    if !is_delegate_registered(deps, &delegate, Some(height))? {
        return Ok(UnvotedDelegatedVotingPowerResponse::default());
    }

    let proposal_module = deps.api.addr_validate(&proposal_module)?;

    let total = get_udvp(deps, &delegate, &proposal_module, proposal_id, height)?;
    let mut effective = total;

    // if a VP cap is set, apply it to the total VP to get the effective VP.
    let vp_cap_percent = VP_CAP_PERCENT
        .may_load_at_height(deps.storage, height)?
        .unwrap_or(None);
    if let Some(vp_cap_percent) = vp_cap_percent {
        if vp_cap_percent < Decimal::one() {
            let dao = DAO.load(deps.storage)?;
            let total_power = voting::get_total_power(deps, &dao, Some(height))?;
            let cap = calculate_delegated_vp(total_power, vp_cap_percent);

            effective = total.min(cap);
        }
    }

    Ok(UnvotedDelegatedVotingPowerResponse { total, effective })
}

fn query_effective_unvoted_delegated_vote_power_reduction(
    deps: Deps,
    proposal_module: String,
    proposal_id: u64,
    proposal_height: u64,
    delegate: String,
    delegated_vp: Uint128,
) -> StdResult<Uint128> {
    let udvp = query_unvoted_delegated_vp(
        deps,
        delegate,
        proposal_module,
        proposal_id,
        proposal_height,
    )?;

    // compute the new effective UDVP after this voter's delegated VP is
    // removed, respecting the configured cap. subtract the voter's delegated VP
    // from the delegate's total UDVP, and cap the result at the delegate's
    // effective UDVP, to ensure we properly take into account the configured VP
    // cap (the effective UDVP is the total UDVP with the cap applied, so the
    // effective UDVP can be used in place of the cap in this computation).
    let new_effective_udvp = udvp.total.checked_sub(delegated_vp)?.min(udvp.effective);

    // compute the amount of UDVP the delegate will lose due to this voter's
    // delegated VP being removed (likely due to a vote override).
    // new_effective_udvp is capped at udvp.effective, so this will never
    // underflow, but use saturating_sub for vibes anyway.
    //
    // the delegate will lose none, part, or all of this voter's delegated VP
    // based on how the delegate's total UDVP and voter's delegated VP compare
    // to the configured cap:
    //
    // 1. if the delegate's total UDVP is less than or equal to the cap, the
    //    delegate will lose all of this voter's delegated VP since the cap is
    //    not exceeded.
    //
    //       IF total_udvp <= cap, THEN loss = voter_delegated
    //
    // 2. if the delegate's total UDVP is greater than the cap by a margin less
    //    than this voter's delegated VP, the delegate will lose part of this
    //    voter's delegated VP. specifically: the difference between the cap and
    //    the delegate's total UDVP without the voter's delegated VP.
    //
    //       IF total_udvp - voter_delegated < cap AND total_udvp > cap, THEN
    //       loss = cap - (total_udvp - voter_delegated)
    //
    // 3. if the delegate's total UDVP is greater than the cap by a margin
    //    greater than or equal to this voter's delegated VP, the delegate will
    //    not lose any VP since the cap is already low enough.
    //
    //       IF total_udvp - voter_delegated >= cap, THEN loss = 0
    Ok(udvp.effective.saturating_sub(new_effective_udvp))
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

fn query_config(deps: Deps) -> StdResult<Config> {
    CONFIG.load(deps.storage)
}

fn query_voting_power_cap(
    deps: Deps,
    env: Env,
    height: Option<u64>,
) -> StdResult<VotingPowerCapResponse> {
    let height = height.unwrap_or(env.block.height);
    let vp_cap_percent = VP_CAP_PERCENT
        .may_load_at_height(deps.storage, height)?
        .flatten();
    Ok(VotingPowerCapResponse {
        vp_cap_percent,
        height,
    })
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
