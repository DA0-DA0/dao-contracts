use cosmwasm_std::{Addr, DepsMut, Env, MessageInfo, Response, StdError, StdResult, Uint128};
use cw4::MemberChangedHookMsg;
use cw_snapshot_vector_map::LoadedItem;
use dao_hooks::{nft_stake::NftStakeChangedHookMsg, stake::StakeChangedHookMsg, vote::VoteHookMsg};

use crate::{
    helpers::{
        calculate_delegated_vp, get_udvp, get_voting_power, is_delegate_registered,
        unregister_delegate,
    },
    state::{
        Delegation, DELEGATED_VP, DELEGATED_VP_AMOUNTS, DELEGATIONS, PROPOSAL_HOOK_CALLERS,
        UNVOTED_DELEGATED_VP, VOTING_POWER_HOOK_CALLERS,
    },
    ContractError,
};

pub(crate) fn execute_stake_changed(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: StakeChangedHookMsg,
) -> Result<Response, ContractError> {
    // ensure voting power hook caller is registered
    if !VOTING_POWER_HOOK_CALLERS.has(deps.storage, info.sender.clone()) {
        return Err(ContractError::UnauthorizedHookCaller {});
    }

    match msg {
        StakeChangedHookMsg::Stake { addr, .. } => {
            handle_voting_power_changed_hook(deps, &env, addr)
        }
        StakeChangedHookMsg::Unstake { addr, .. } => {
            handle_voting_power_changed_hook(deps, &env, addr)
        }
    }
}

pub(crate) fn execute_membership_changed(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: MemberChangedHookMsg,
) -> Result<Response, ContractError> {
    // ensure voting power hook caller is registered
    if !VOTING_POWER_HOOK_CALLERS.has(deps.storage, info.sender.clone()) {
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
    info: MessageInfo,
    msg: NftStakeChangedHookMsg,
) -> Result<Response, ContractError> {
    // ensure voting power hook caller is registered
    if !VOTING_POWER_HOOK_CALLERS.has(deps.storage, info.sender.clone()) {
        return Err(ContractError::UnauthorizedHookCaller {});
    }

    match msg {
        NftStakeChangedHookMsg::Stake { addr, .. } => {
            handle_voting_power_changed_hook(deps, &env, addr)
        }
        NftStakeChangedHookMsg::Unstake { addr, .. } => {
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
    let new_vp = get_voting_power(
        deps.as_ref(),
        &addr,
        // use next block height since voting power takes effect at the start of
        // the next block. since the member changed their voting power in the
        // current block, we need to use the new value.
        env.block.height + 1,
    )?;

    // check latest state instead of historical height, since we need access to
    // immediate updates made earlier in the same block
    if is_delegate_registered(deps.as_ref(), &addr, None)? {
        let delegate = addr;

        // unregister if no more voting power
        if new_vp.is_zero() {
            unregister_delegate(deps, &delegate, env.block.height)?;
        }
    }
    // if not a delegate, check if they have any delegations, and update
    // delegate VPs accordingly
    else {
        let delegator = addr;

        // need to get the latest delegations in case any were updated earlier
        // in the same block
        let delegations =
            DELEGATIONS.load_all_latest(deps.storage, &delegator, env.block.height)?;

        for LoadedItem {
            item: Delegation { delegate, percent },
            ..
        } in delegations
        {
            // remove the current delegated VP from the delegate's total and
            // replace it with the new delegated VP
            let current_delegated_vp =
                DELEGATED_VP_AMOUNTS.load(deps.storage, (&delegator, &delegate))?;
            let new_delegated_vp = calculate_delegated_vp(new_vp, percent);

            // this `update` function loads the latest delegated VP, even if it
            // was updated before in this block, and then saves the new total at
            // the current block, which will be reflected in historical queries
            // starting from the NEXT block. if future
            // delegations/undelegations/voting power changes occur in this
            // block, they will immediately load the latest state, and update
            // the total that will be reflected in historical queries starting
            // from the next block.
            DELEGATED_VP.update(
                deps.storage,
                &delegate,
                env.block.height,
                |vp| -> StdResult<Uint128> {
                    Ok(vp
                        .unwrap_or_default()
                        .checked_sub(current_delegated_vp)
                        .map_err(StdError::overflow)?
                        .checked_add(new_delegated_vp)
                        .map_err(StdError::overflow)?)
                },
            )?;
            DELEGATED_VP_AMOUNTS.save(deps.storage, (&delegator, &delegate), &new_delegated_vp)?;
        }
    }

    Ok(Response::new().add_attribute("action", "voting_power_change_hook"))
}

pub fn execute_vote_hook(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    vote_hook: VoteHookMsg,
) -> Result<Response, ContractError> {
    let proposal_module = info.sender;

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
