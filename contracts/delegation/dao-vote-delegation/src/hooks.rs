use cosmwasm_std::{Addr, DepsMut, Env, Response, Uint128};
use cw4::MemberChangedHookMsg;
use cw_snapshot_vector_map::LoadedItem;
use dao_hooks::{nft_stake::NftStakeChangedHookMsg, stake::StakeChangedHookMsg, vote::VoteHookMsg};
use dao_voting::delegation::calculate_delegated_vp;

use crate::{
    helpers::{
        add_delegated_vp, get_udvp, is_delegate_registered, remove_delegated_vp,
        unregister_delegate,
    },
    state::{
        Delegation, CONFIG, DAO, DELEGATIONS, PROPOSAL_HOOK_CALLERS, UNVOTED_DELEGATED_VP,
        VOTING_POWER_HOOK_CALLERS,
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

    let addr = match msg {
        StakeChangedHookMsg::Stake { addr, .. } | StakeChangedHookMsg::Unstake { addr, .. } => addr,
    };

    handle_voting_power_changed_hook(deps, &env, addr)
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

    let addr = match msg {
        NftStakeChangedHookMsg::Stake { addr, .. }
        | NftStakeChangedHookMsg::Unstake { addr, .. } => addr,
    };

    handle_voting_power_changed_hook(deps, &env, addr)
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

    // check latest state instead of historical height, since we need access to
    // immediate updates made earlier in the same block
    match is_delegate_registered(deps.as_ref(), &addr, None)? {
        true => handle_delegate_voting_power_changed_hook(deps, env.block.height, addr, new_vp),
        // if not a delegate, check if they have any delegations, and update
        // delegate VPs accordingly
        false => handle_delegator_voting_power_changed_hook(deps, env, dao, addr, new_vp),
    }
}

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
    let old_vp = dao_voting::voting::get_voting_power(deps.as_ref(), delegator, &dao, None)?;

    for LoadedItem {
        item: Delegation { delegate, percent },
        expiration,
        ..
    } in delegations
    {
        // remove the latest delegated VP from the delegate's total and
        // replace it with the new delegated VP

        // remove original delegated VP if nonzero
        let current_delegated_vp = calculate_delegated_vp(old_vp, percent);
        if !current_delegated_vp.is_zero() {
            remove_delegated_vp(
                deps.storage,
                env,
                &delegate,
                current_delegated_vp,
                expiration,
            )?;
        }

        // add new delegated VP if nonzero
        let new_delegated_vp = calculate_delegated_vp(new_vp, percent);
        if !new_delegated_vp.is_zero() {
            add_delegated_vp(
                deps.storage,
                env,
                &delegate,
                new_delegated_vp,
                config.delegation_validity_blocks,
            )?;
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
            height,
            is_first_vote,
            ..
        } => {
            // if first vote, update the unvoted delegated VP for their
            // delegates by subtracting this member's delegated VP. if not first
            // vote, this has already been done.
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
