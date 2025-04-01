use cosmwasm_std::{Addr, DepsMut, Env, Response, Uint128};
use cw4::MemberChangedHookMsg;
use cw_snapshot_vector_map::LoadedItem;
use dao_hooks::{nft_stake::NftStakeChangedHookMsg, stake::StakeChangedHookMsg, vote::VoteHookMsg};
use dao_voting::delegation::calculate_delegated_vp;
use std::cmp::Ordering;

use crate::{
    helpers::{get_udvp, is_delegate_registered, unregister_delegate},
    state::{
        Delegation, CONFIG, DAO, DELEGATED_VP, DELEGATIONS, PROPOSAL_HOOK_CALLERS,
        UNVOTED_DELEGATED_VP, VOTING_POWER_HOOK_CALLERS,
    },
    ContractError,
};

pub(crate) fn execute_stake_changed(
    deps: DepsMut,
    env: Env,
    sender: Addr,
    msg: StakeChangedHookMsg,
) -> Result<Response, ContractError> {
    // ensure voting power hook caller is registered
    if !VOTING_POWER_HOOK_CALLERS.has(deps.storage, sender) {
        return Err(ContractError::UnauthorizedHookCaller {});
    }

    match msg {
        StakeChangedHookMsg::Stake { addr, .. } | StakeChangedHookMsg::Unstake { addr, .. } => {
            handle_voting_power_changed_hook(deps, &env, addr)
        }
    }
}

pub(crate) fn execute_membership_changed(
    mut deps: DepsMut,
    env: Env,
    sender: Addr,
    msg: MemberChangedHookMsg,
) -> Result<Response, ContractError> {
    // ensure voting power hook caller is registered
    if !VOTING_POWER_HOOK_CALLERS.has(deps.storage, sender) {
        return Err(ContractError::UnauthorizedHookCaller {});
    }

    // Get the members whose voting power changed and update their voting power.
    for member in msg.diffs {
        let addr = deps.api.addr_validate(&member.key)?;
        handle_voting_power_changed_hook(deps.branch(), &env, addr)?;
    }

    Ok(Response::new().add_attribute("action", "voting_power_change_hook"))
}

pub(crate) fn execute_nft_stake_changed(
    deps: DepsMut,
    env: Env,
    sender: Addr,
    msg: NftStakeChangedHookMsg,
) -> Result<Response, ContractError> {
    // ensure voting power hook caller is registered
    if !VOTING_POWER_HOOK_CALLERS.has(deps.storage, sender) {
        return Err(ContractError::UnauthorizedHookCaller {});
    }

    match msg {
        NftStakeChangedHookMsg::Stake { addr, .. }
        | NftStakeChangedHookMsg::Unstake { addr, .. } => {
            handle_voting_power_changed_hook(deps, &env, addr)
        }
    }
}

/// Perform necessary updates when a member's voting power changes.
pub(crate) fn handle_voting_power_changed_hook(
    deps: DepsMut,
    env: &Env,
    addr: Addr,
) -> Result<Response, ContractError> {
    let dao = DAO.load(deps.storage)?;

    let new_vp = dao_voting::voting::get_voting_power(
        deps.as_ref(),
        addr.clone(),
        &dao,
        // use next block height since voting power takes effect at the start of
        // the next block. since the member changed their voting power in the
        // current block, we need to use the new value.
        Some(env.block.height + 1),
    )?;

    // depending on whether the voting power hook was fired for a delegate or
    // delegator, we need to handle the voting power change differently. check
    // latest state instead of historical height, since we need access to
    // immediate updates made earlier in the same block
    if is_delegate_registered(deps.as_ref(), &addr, None)? {
        handle_delegate_voting_power_changed_hook(deps, env, addr, new_vp)
    } else {
        // if not a delegate, check if they have any delegations, and update
        // delegate VPs accordingly
        handle_delegator_voting_power_changed_hook(deps, env, dao, addr, new_vp)
    }
}

/// handles the delegate voting power changed hook by unregistering the delegate
/// if they no longer have any voting power.
fn handle_delegate_voting_power_changed_hook(
    deps: DepsMut,
    env: &Env,
    delegate: Addr,
    new_vp: Uint128,
) -> Result<Response, ContractError> {
    // unregister if no more voting power
    if new_vp.is_zero() {
        unregister_delegate(deps, &delegate, env.block.height)?;
    }

    Ok(Response::new()
        .add_attribute("action", "voting_power_change_hook")
        .add_attribute("member_type", "delegate"))
}

/// handles the delegator voting power changed hook by updating their delegated
/// VP for each delegate they are delegating to and each delegate's total
/// delegated VP.
fn handle_delegator_voting_power_changed_hook(
    deps: DepsMut,
    env: &Env,
    dao: Addr,
    delegator: Addr,
    new_vp: Uint128,
) -> Result<Response, ContractError> {
    // need to get the latest delegations in case any were updated earlier in
    // the same block
    let delegations = DELEGATIONS.load_all_latest(deps.storage, &delegator, env.block.height)?;

    let config = CONFIG.load(deps.storage)?;
    let old_vp = dao_voting::voting::get_voting_power(
        deps.as_ref(),
        delegator,
        &dao,
        Some(env.block.height),
    )?;

    for LoadedItem {
        item: Delegation { delegate, percent },
        expiration,
        ..
    } in delegations
    {
        // for each delegation, we first find the current delegated VP and the
        // new delegated VP
        let current_delegated_vp = calculate_delegated_vp(old_vp, percent);
        let new_delegated_vp = calculate_delegated_vp(new_vp, percent);

        // first we update the next block's delegated VP for the delegate
        match new_delegated_vp.cmp(&current_delegated_vp) {
            Ordering::Less => {
                // if the new delegated VP is less than the current delegated
                // VP, we decrement the delegated VP by the delta.
                DELEGATED_VP.decrement(
                    deps.storage,
                    delegate.clone(),
                    // update at next block height to match 1-block delay
                    // behavior of voting power queries and delegation changes.
                    // this matches the behavior of creating a new delegation,
                    // which also starts on the following block. if future
                    // delegations/undelegations/voting power changes occur in
                    // this block, they will also load the state of the next
                    // block and update the total that will be reflected in
                    // historical queries starting from the next block.
                    env.block.height + 1,
                    current_delegated_vp - new_delegated_vp,
                )?;
            }
            Ordering::Equal => {
                // for cases where current delegated VP is equal to new
                // delegated VP, we don't need to do anything.
            }
            Ordering::Greater => {
                // if the new delegated VP is greater than the current delegated
                // VP, we increment the delegated VP by the delta.
                DELEGATED_VP.increment(
                    deps.storage,
                    delegate.clone(),
                    // update at next block height to match 1-block delay
                    // behavior of voting power queries and delegation changes.
                    // this matches the behavior of creating a new delegation,
                    // which also starts on the following block. if future
                    // delegations/undelegations/voting power changes occur in
                    // this block, they will also load the state of the next
                    // block and update the total that will be reflected in
                    // historical queries starting from the next block.
                    env.block.height + 1,
                    new_delegated_vp - current_delegated_vp,
                )?;
            }
        }

        // if original delegation had voting power and an expiration date, we
        // undo the previous decrement at the end of the expiration period.
        if current_delegated_vp.u128() > 0 {
            if let Some(expire_in) = expiration {
                DELEGATED_VP.increment(
                    deps.storage,
                    delegate.clone(),
                    expire_in,
                    current_delegated_vp,
                )?;
            }
        }

        // if the new delegation has voting power and global config specifies a
        // delegation validity duration, we apply the decrement at the end of
        // the expiration period.
        if new_delegated_vp.u128() > 0 {
            if let Some(config_expiration) = config.delegation_validity_blocks {
                DELEGATED_VP.decrement(
                    deps.storage,
                    delegate.clone(),
                    env.block.height + config_expiration,
                    new_delegated_vp,
                )?;
            }
        }
    }

    Ok(Response::new()
        .add_attribute("action", "voting_power_change_hook")
        .add_attribute("member_type", "delegator"))
}

// if first vote by a delegator, update the unvoted delegated VP for their
// delegates, if any, by subtracting this member's delegated VP. if not first
// vote, this has already been done.
pub fn execute_vote_hook(
    deps: DepsMut,
    proposal_module: Addr,
    vote_hook: VoteHookMsg,
) -> Result<Response, ContractError> {
    // ensure proposal module is registered
    if !PROPOSAL_HOOK_CALLERS.has(deps.storage, proposal_module.clone()) {
        return Err(ContractError::UnauthorizedHookCaller {});
    }

    match vote_hook {
        VoteHookMsg::NewVote {
            proposal_id,
            voter,
            power,
            height,
            is_first_vote,
            ..
        } => {
            // if not first vote, this has already been done.
            if !is_first_vote {
                return Ok(Response::new()
                    .add_attribute("action", "vote_hook")
                    .add_attribute("is_first_vote", "false"));
            }

            // update voting power for all delegates, if any. if this voter is a
            // delegate themself, there will simply be no delegations.
            let delegator = deps.api.addr_validate(&voter)?;
            let delegations = DELEGATIONS.load_all(deps.storage, &delegator, height)?;
            for LoadedItem {
                item: Delegation { delegate, percent },
                ..
            } in delegations
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

    Ok(Response::new()
        .add_attribute("action", "vote_hook")
        .add_attribute("is_first_vote", "true"))
}
