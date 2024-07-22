use cosmwasm_std::{Addr, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Storage};
use cw4::MemberChangedHookMsg;
use dao_hooks::{nft_stake::NftStakeChangedHookMsg, stake::StakeChangedHookMsg};

use crate::{rewards::update_rewards, state::REGISTERED_HOOKS, ContractError};

/// Register a hook caller contract for a given distribution ID.
pub(crate) fn subscribe_distribution_to_hook(
    storage: &mut dyn Storage,
    distribution_id: u64,
    hook: Addr,
) -> Result<(), ContractError> {
    REGISTERED_HOOKS.update(storage, hook, |denoms| -> StdResult<_> {
        let mut denoms = denoms.unwrap_or_default();
        denoms.push(distribution_id);
        Ok(denoms)
    })?;
    Ok(())
}

/// Unregister a hook caller contract for a given distribution ID.
pub(crate) fn unsubscribe_distribution_from_hook(
    storage: &mut dyn Storage,
    distribution_id: u64,
    hook: Addr,
) -> Result<(), ContractError> {
    let mut denoms = REGISTERED_HOOKS
        .may_load(storage, hook.clone())?
        .unwrap_or_default();

    denoms.retain(|id| *id != distribution_id);

    if denoms.is_empty() {
        REGISTERED_HOOKS.remove(storage, hook);
    } else {
        REGISTERED_HOOKS.save(storage, hook, &denoms)?;
    }

    Ok(())
}

/// Ensures hooks that update voting power are only called by a designated
/// hook_caller contract.
/// Returns a list of distribution IDs that the hook caller is registered for.
pub(crate) fn get_hook_caller_registered_distribution_ids(
    deps: Deps,
    info: MessageInfo,
) -> Result<Vec<u64>, ContractError> {
    // only a designated hook_caller contract can call this hook.
    // failing to load the registered denoms for a given hook returns an error.
    REGISTERED_HOOKS
        .load(deps.storage, info.sender.clone())
        .map_err(|_| ContractError::InvalidHookSender {})
}

pub(crate) fn execute_stake_changed(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: StakeChangedHookMsg,
) -> Result<Response, ContractError> {
    // Check that the sender is the vp_contract (or the hook_caller if configured).
    let hooked_distribution_ids = get_hook_caller_registered_distribution_ids(deps.as_ref(), info)?;

    match msg {
        StakeChangedHookMsg::Stake { addr, .. } => {
            update_for_stake(deps, env, addr, hooked_distribution_ids)
        }
        StakeChangedHookMsg::Unstake { addr, .. } => {
            execute_unstake(deps, env, addr, hooked_distribution_ids)
        }
    }
}

pub(crate) fn execute_membership_changed(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: MemberChangedHookMsg,
) -> Result<Response, ContractError> {
    // Check that the sender is the vp_contract (or the hook_caller if configured).
    let hooked_distribution_ids = get_hook_caller_registered_distribution_ids(deps.as_ref(), info)?;

    // Get the addresses of members whose voting power has changed.
    for member in msg.diffs {
        let addr = deps.api.addr_validate(&member.key)?;
        for id in hooked_distribution_ids.clone() {
            update_rewards(&mut deps, &env, &addr, id)?;
        }
    }

    Ok(Response::new().add_attribute("action", "membership_changed"))
}

pub(crate) fn execute_nft_stake_changed(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: NftStakeChangedHookMsg,
) -> Result<Response, ContractError> {
    // Check that the sender is the vp_contract (or the hook_caller if configured).
    let hooked_distribution_ids = get_hook_caller_registered_distribution_ids(deps.as_ref(), info)?;

    match msg {
        NftStakeChangedHookMsg::Stake { addr, .. } => {
            update_for_stake(deps, env, addr, hooked_distribution_ids)
        }
        NftStakeChangedHookMsg::Unstake { addr, .. } => {
            execute_unstake(deps, env, addr, hooked_distribution_ids)
        }
    }
}

pub(crate) fn update_for_stake(
    mut deps: DepsMut,
    env: Env,
    addr: Addr,
    hooked_distribution_ids: Vec<u64>,
) -> Result<Response, ContractError> {
    // update rewards for every distribution ID that the hook caller is
    // registered for
    for id in hooked_distribution_ids {
        update_rewards(&mut deps, &env, &addr, id)?;
    }
    Ok(Response::new().add_attribute("action", "stake"))
}

pub(crate) fn execute_unstake(
    mut deps: DepsMut,
    env: Env,
    addr: Addr,
    hooked_distribution_ids: Vec<u64>,
) -> Result<Response, ContractError> {
    // update rewards for every distribution ID that the hook caller is
    // registered for
    for id in hooked_distribution_ids {
        update_rewards(&mut deps, &env, &addr, id)?;
    }
    Ok(Response::new().add_attribute("action", "unstake"))
}
