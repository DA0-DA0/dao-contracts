use cosmwasm_std::{Addr, Decimal, Deps, DepsMut, Env, StdError, StdResult, Storage, Uint128};

use dao_voting::{
    delegation::{calculate_delegated_vp, Config, Delegation},
    voting,
};

use crate::{
    state::{
        DAO, DELEGATED_VP, DELEGATES, DELEGATIONS, PERCENT_DELEGATED, PROPOSAL_HOOK_CALLERS,
        UNVOTED_DELEGATED_VP, VOTING_POWER_HOOK_CALLERS,
    },
    ContractError,
};

pub type DelegationHandlerResult = Result<(Decimal, (u64, Option<u64>), bool), ContractError>;

pub fn unregister_delegate(deps: DepsMut, delegate: &Addr, height: u64) -> StdResult<()> {
    DELEGATES.remove(deps.storage, delegate.clone(), height)
}

pub fn is_delegate_registered(deps: Deps, delegate: &Addr, height: Option<u64>) -> StdResult<bool> {
    let option = if let Some(height) = height {
        DELEGATES.may_load_at_height(deps.storage, delegate.clone(), height)
    } else {
        DELEGATES.may_load(deps.storage, delegate.clone())
    };

    option.map(|d| d.is_some())
}

pub fn get_voting_power(deps: Deps, addr: &Addr, height: u64) -> StdResult<Uint128> {
    let dao = DAO.load(deps.storage)?;
    voting::get_voting_power(deps, addr.clone(), &dao, Some(height))
}

/// Returns the unvoted delegated VP for a delegate on a proposal, falling back
/// to the delegate's total delegated VP at the given height if no unvoted
/// delegated VP exists for the proposal.
///
/// **NOTE: The caller is responsible for ensuring that the block height
/// corresponds to the correct height for the proposal.**
pub fn get_udvp(
    deps: Deps,
    delegate: &Addr,
    proposal_module: &Addr,
    proposal_id: u64,
    proposal_height: u64,
) -> StdResult<Uint128> {
    // if no unvoted delegated VP exists for the proposal, use the delegate's
    // total delegated VP at that height. UNVOTED_DELEGATED_VP gets set when one
    // of their delegators casts a vote. if empty, none of them have voted yet.
    match UNVOTED_DELEGATED_VP.may_load(deps.storage, (delegate, proposal_module, proposal_id))? {
        Some(vp) => Ok(vp),
        None => Ok(DELEGATED_VP
            .load(deps.storage, delegate.clone(), proposal_height)?
            .unwrap_or_default()),
    }
}

/// Ensures the delegation module is setup correctly.
pub fn ensure_setup(deps: Deps) -> Result<(), ContractError> {
    if VOTING_POWER_HOOK_CALLERS.is_empty(deps.storage)
        || PROPOSAL_HOOK_CALLERS.is_empty(deps.storage)
    {
        return Err(ContractError::DelegationModuleNotSetup {});
    }

    Ok(())
}

/// Ensures that the max delegations limit has not been reached.
pub fn ensure_max_delegations_not_reached(
    max: u64,
    old: usize,
    new: usize,
) -> Result<(), ContractError> {
    if new > max as usize {
        return Err(ContractError::MaxDelegationsReached { max, current: old });
    }

    Ok(())
}

/// Add delegated VP from a delegator to a delegate, potentially with a given
/// expiration.
pub fn add_delegated_vp(
    storage: &mut dyn Storage,
    env: &Env,
    delegate: &Addr,
    vp: Uint128,
    expiration: Option<u64>,
) -> StdResult<()> {
    DELEGATED_VP.increment(
        storage,
        delegate.clone(),
        // update at next block height to match 1-block delay behavior of voting
        // power queries and delegation changes. this matches the behavior of
        // creating a new delegation, which also starts on the following block.
        // if future delegations/undelegations/voting power changes occur in
        // this block, they will also load the state of the next block and
        // update the total that will be reflected in historical queries
        // starting from the next block.
        env.block.height + 1,
        vp,
    )?;

    // if expiration exists, decrement in the future at expiration height
    if let Some(expiration) = expiration {
        DELEGATED_VP.decrement(storage, delegate.clone(), expiration, vp)?;
    }

    Ok(())
}

/// Remove delegated VP from a delegate, potentially with a given expiration, if
/// not already expired. If already expired, the delegated VP should already be
/// removed due to Wormhole's future decrement behavior (in `add_delegated_vp`
/// above).
pub fn remove_delegated_vp_if_not_expired(
    storage: &mut dyn Storage,
    env: &Env,
    delegate: &Addr,
    vp: Uint128,
    original_expiration: Option<u64>,
) -> StdResult<()> {
    // if delegation already expired, do nothing.
    if let Some(original_expiration) = original_expiration {
        if original_expiration <= env.block.height {
            return Ok(());
        }
    }

    // if expiration was used when creating this delegation, first undo previous
    // decrement at end of expiration period. do this before undoing previous
    // increment to prevent underflow.
    if let Some(original_expiration) = original_expiration {
        DELEGATED_VP.increment(storage, delegate.clone(), original_expiration, vp)?;
    }

    DELEGATED_VP.decrement(
        storage,
        delegate.clone(),
        // update at next block height to match 1-block delay behavior of voting
        // power queries and delegation changes. this matches the behavior of
        // creating a new delegation, which also starts on the following block.
        // if future delegations/undelegations/voting power changes occur in
        // this block, they will also load the state of the next block and
        // update the total that will be reflected in historical queries
        // starting from the next block.
        env.block.height + 1,
        vp,
    )?;

    Ok(())
}

/// Update delegated VP expiration, erroring if either is in the past.
pub fn update_delegated_vp_expiration(
    storage: &mut dyn Storage,
    env: &Env,
    delegate: &Addr,
    vp: Uint128,
    original_expiration: Option<u64>,
    new_expiration: Option<u64>,
) -> StdResult<()> {
    // if expiration was used when creating this delegation, first undo previous
    // decrement at end of expiration period.
    if let Some(original_expiration) = original_expiration {
        if original_expiration <= env.block.height {
            return Err(StdError::generic_err(
                "original expiration is in the past, cannot rewrite history",
            ));
        }

        DELEGATED_VP.increment(storage, delegate.clone(), original_expiration, vp)?;
    }

    // if new expiration is set, decrement at new expiration
    if let Some(new_expiration) = new_expiration {
        if new_expiration <= env.block.height {
            return Err(StdError::generic_err(
                "new expiration is in the past, cannot rewrite history",
            ));
        }

        DELEGATED_VP.decrement(storage, delegate.clone(), new_expiration, vp)?;
    }

    Ok(())
}

/// Validates a delegation and returns the delegator's voting power if valid.
pub fn validate_delegation(
    deps: Deps,
    env: &Env,
    delegator: &Addr,
    delegate: &Addr,
    new_percent: Decimal,
) -> Result<Uint128, ContractError> {
    ensure_setup(deps)?;

    if new_percent <= Decimal::zero() || new_percent > Decimal::one() {
        return Err(ContractError::InvalidVotingPowerPercent {});
    }

    // delegates cannot delegate to others
    if is_delegate_registered(deps, delegator, None)? {
        return Err(ContractError::DelegatesCannotDelegate {});
    }

    // ensure delegate is registered
    if !is_delegate_registered(deps, delegate, None)? {
        return Err(ContractError::DelegateNotRegistered {});
    }

    // ensure delegator has voting power in the DAO
    let vp = get_voting_power(
        deps,
        delegator,
        // use next block height since voting power takes effect at the start of
        // the next block. if the delegator changed their voting power in the
        // current block, we need to use the new value.
        env.block.height + 1,
    )?;
    if vp.is_zero() {
        return Err(ContractError::NoVotingPower {});
    }

    Ok(vp)
}

/// Handle redelegation by updating the existing delegation.
#[allow(clippy::too_many_arguments)]
pub fn handle_redelegation(
    deps: DepsMut,
    env: &Env,
    delegator: &Addr,
    delegate: &Addr,
    new_percent: Decimal,
    config: &Config,
    current_percent_delegated: Decimal,
    vp: Uint128,
    existing_delegation_entry: (u64, Option<u64>),
) -> DelegationHandlerResult {
    let (existing_delegation_id, existing_delegation_expiration) = existing_delegation_entry;
    let expired = existing_delegation_expiration.is_some_and(|exp| exp <= env.block.height);

    let existing_delegation_percent = DELEGATIONS
        .load_item(deps.storage, delegator, existing_delegation_id)?
        .percent;

    // if delegation is not expired and percent is the same, just extend the
    // expiration based on the current config.
    if !expired && existing_delegation_percent == new_percent {
        // if both expirations are none, do nothing.
        if existing_delegation_expiration.is_none() && config.delegation_validity_blocks.is_none() {
            return Ok((
                current_percent_delegated,
                (existing_delegation_id, existing_delegation_expiration),
                false,
            ));
        }

        let new_expiration = DELEGATIONS.update_expiration(
            deps.storage,
            delegator,
            existing_delegation_id,
            env.block.height,
            config.delegation_validity_blocks,
        )?;

        update_delegated_vp_expiration(
            deps.storage,
            env,
            delegate,
            vp,
            existing_delegation_expiration,
            new_expiration,
        )?;

        return Ok((
            current_percent_delegated,
            (existing_delegation_id, new_expiration),
            false,
        ));
    }

    // remove existing percent and replace with new percent
    let new_total = current_percent_delegated
        .checked_sub(existing_delegation_percent)?
        .checked_add(new_percent)?;

    if !expired {
        // remove current delegated VP based on existing percent
        let old_vp = calculate_delegated_vp(vp, existing_delegation_percent);
        remove_delegated_vp_if_not_expired(
            deps.storage,
            env,
            delegate,
            old_vp,
            existing_delegation_expiration,
        )?;
    }

    // update the delegation percent
    let (entry, total_count) = DELEGATIONS.update(
        deps.storage,
        delegator,
        existing_delegation_id,
        env.block.height,
        |d| {
            d.percent = new_percent;
        },
        config.delegation_validity_blocks,
    )?;

    // don't allow update if over the max, instead requiring them to remove
    // existing delegations before updating any
    ensure_max_delegations_not_reached(config.max_delegations, total_count, total_count)?;

    Ok((new_total, entry, true))
}

/// Handle new delegation.
pub fn handle_new_delegation(
    deps: DepsMut,
    env: &Env,
    delegator: &Addr,
    delegate: &Addr,
    new_percent: Decimal,
    config: &Config,
    current_percent_delegated: Decimal,
) -> DelegationHandlerResult {
    let new_total = current_percent_delegated.checked_add(new_percent)?;

    // add new delegation
    let (entry, total_count) = DELEGATIONS.push(
        deps.storage,
        delegator,
        &Delegation {
            delegate: delegate.clone(),
            percent: new_percent,
        },
        env.block.height,
        config.delegation_validity_blocks,
    )?;

    // prevent new delegations if over the max
    ensure_max_delegations_not_reached(config.max_delegations, total_count - 1, total_count)?;

    Ok((new_total, entry, true))
}

/// Validate and update total percent delegated for a delegator.
pub fn validate_and_update_percent_delegated(
    deps: DepsMut,
    delegator: &Addr,
    current_total_percent: Decimal,
    new_total_percent: Decimal,
) -> Result<(), ContractError> {
    // ensure not delegating more than 100%
    if new_total_percent > Decimal::one() {
        return Err(ContractError::CannotDelegateMoreThan100Percent {
            // multiply decimal (between 0 and 1) by 100 (which = 10,000%) to
            // convert to a human-readable percentage out of 100%
            current: current_total_percent
                .checked_mul(Decimal::percent(10_000))?
                .to_string(),
            // multiply decimal (between 0 and 1) by 100 (which = 10,000%) to
            // convert to a human-readable percentage out of 100%
            attempt: new_total_percent
                .checked_mul(Decimal::percent(10_000))?
                .to_string(),
        });
    }

    // final state updates applicable to both new and existing delegations
    PERCENT_DELEGATED.save(deps.storage, delegator, &new_total_percent)?;

    Ok(())
}
