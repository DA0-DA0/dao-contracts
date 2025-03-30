use cosmwasm_std::{Addr, DepsMut, Env, Response, Uint128};
use cw4::MemberChangedHookMsg;
use cw_snapshot_vector_map::LoadedItem;
use dao_hooks::{nft_stake::NftStakeChangedHookMsg, stake::StakeChangedHookMsg, vote::VoteHookMsg};
use dao_voting::delegation::calculate_delegated_vp;

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
///
/// For delegators:
/// - update their delegated VP for each delegate
/// - update each delegate's total delegated VP
///
/// For delegates:
/// - unregister them if they have no voting power
/// - TODO: re-register them if previously registered but had no voting power???
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

    // depending on whether the voting power hook was fired for a delegate or delegator,
    // we need to handle the voting power change differently.
    // check latest state instead of historical height, since we need access to
    // immediate updates made earlier in the same block
    if is_delegate_registered(deps.as_ref(), &addr, None)? {
        handle_delegate_voting_power_changed_hook(deps, env.block.height, addr, new_vp)
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
    block_height: u64,
    delegate: Addr,
    new_vp: Uint128,
) -> Result<Response, ContractError> {
    // unregister if no more voting power
    if new_vp.is_zero() {
        unregister_delegate(deps, &delegate, block_height)?;
    }

    Ok(Response::new()
        .add_attribute("action", "voting_power_change_hook")
        .add_attribute("member_type", "delegate"))
}

/// handles the delegator voting power changed hook by updating
fn handle_delegator_voting_power_changed_hook(
    deps: DepsMut,
    env: &Env,
    dao: Addr,
    delegator: Addr,
    new_vp: Uint128,
) -> Result<Response, ContractError> {
    // need to get the latest delegations in case any were updated earlier
    // in the same block
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
        // for each delegation, we first find the current delegated VP and
        // the new delegated VP
        let current_delegated_vp = calculate_delegated_vp(old_vp, percent);
        let new_delegated_vp = calculate_delegated_vp(new_vp, percent);

        // first we update the next block's delegated VP for the delegate.
        // for cases where current delegated VP is equal to new delegated VP,
        // we don't need to do anything.
        if new_delegated_vp > current_delegated_vp {
            // if the new delegated VP is greater than the current delegated VP,
            // we increment the delegated VP by the delta.
            DELEGATED_VP.increment(
                deps.storage,
                delegate.clone(),
                env.block.height + 1,
                new_delegated_vp - current_delegated_vp,
            )?;
        } else if current_delegated_vp > new_delegated_vp {
            // if the new delegated VP is lesser than the current delegated VP,
            // we decrement the delegated VP by the delta.
            DELEGATED_VP.decrement(
                deps.storage,
                delegate.clone(),
                env.block.height + 1,
                current_delegated_vp - new_delegated_vp,
            )?;
        }

        // if original delegation had voting power and an expiration date,
        // we undo the previous decrement at the end of the expiration period.
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

        // if the new delegation has voting power & global config specifies
        // a delegation validity duration, we apply the decrement at the end
        // of the expiration period.
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

pub fn execute_vote_hook(
    deps: DepsMut,
    env: Env,
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
            is_first_vote,
            ..
        } => {
            // if first vote, update the unvoted delegated VP for their
            // delegates by subtracting this member's delegated VP. if not first
            // vote, this has already been done.
            if is_first_vote {
                handle_first_delegator_vote(
                    deps,
                    voter,
                    env.block.height,
                    power,
                    proposal_id,
                    &proposal_module,
                )?;
            }
        }
    }

    Ok(Response::new().add_attribute("action", "vote_hook"))
}

fn handle_first_delegator_vote(
    deps: DepsMut,
    voter: String,
    env_block_height: u64,
    power: Uint128,
    proposal_id: u64,
    proposal_module: &Addr,
) -> Result<(), ContractError> {
    let delegator = deps.api.addr_validate(&voter)?;
    let delegations = DELEGATIONS.load_all(deps.storage, &delegator, env_block_height)?;
    for LoadedItem {
        item: Delegation { delegate, percent },
        ..
    } in delegations
    {
        let udvp = get_udvp(
            deps.as_ref(),
            &delegate,
            proposal_module,
            proposal_id,
            env_block_height,
        )?;

        let delegated_vp = calculate_delegated_vp(power, percent);

        // remove the delegator's delegated VP from the delegate's
        // unvoted delegated VP for this proposal since this
        // delegator just voted.
        let new_udvp = udvp.checked_sub(delegated_vp)?;

        UNVOTED_DELEGATED_VP.save(
            deps.storage,
            (&delegate, proposal_module, proposal_id),
            &new_udvp,
        )?;
    }

    Ok(())
}
