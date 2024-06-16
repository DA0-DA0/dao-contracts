use cosmwasm_std::{Addr, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw4::MemberChangedHookMsg;
use dao_hooks::{nft_stake::NftStakeChangedHookMsg, stake::StakeChangedHookMsg};

use crate::{contract::update_rewards, state::REGISTERED_HOOK_DENOMS, ContractError};

/// Register a hook caller contract for a given denom.
pub(crate) fn subscribe_denom_to_hook(
    deps: DepsMut,
    denom: String,
    hook: Addr,
) -> Result<(), ContractError> {
    REGISTERED_HOOK_DENOMS.update(deps.storage, hook, |denoms| -> StdResult<_> {
        let mut denoms = denoms.unwrap_or_default();
        denoms.push(denom.to_string());
        Ok(denoms)
    })?;
    Ok(())
}

/// Ensures hooks that update voting power are only called by a designated
/// hook_caller contract.
/// Returns a list of denoms that the hook caller is registered for.
pub(crate) fn get_hook_caller_registered_denoms(
    deps: Deps,
    info: MessageInfo,
) -> Result<Vec<String>, ContractError> {
    // only a designated hook_caller contract can call this hook.
    // failing to load the registered denoms for a given hook returns an error.
    REGISTERED_HOOK_DENOMS
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
    let hooked_denoms = get_hook_caller_registered_denoms(deps.as_ref(), info)?;

    match msg {
        StakeChangedHookMsg::Stake { addr, .. } => execute_stake(deps, env, addr, hooked_denoms),
        StakeChangedHookMsg::Unstake { addr, .. } => {
            execute_unstake(deps, env, addr, hooked_denoms)
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
    let hooked_denoms = get_hook_caller_registered_denoms(deps.as_ref(), info)?;

    // Get the addresses of members whose voting power has changed.
    for member in msg.diffs {
        let addr = deps.api.addr_validate(&member.key)?;
        for denom in hooked_denoms.clone() {
            update_rewards(&mut deps, &env, &addr, denom)?;
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
    let hooked_denoms = get_hook_caller_registered_denoms(deps.as_ref(), info)?;

    match msg {
        NftStakeChangedHookMsg::Stake { addr, .. } => execute_stake(deps, env, addr, hooked_denoms),
        NftStakeChangedHookMsg::Unstake { addr, .. } => {
            execute_unstake(deps, env, addr, hooked_denoms)
        }
    }
}

pub(crate) fn execute_stake(
    mut deps: DepsMut,
    env: Env,
    addr: Addr,
    hooked_denoms: Vec<String>,
) -> Result<Response, ContractError> {
    // update rewards for every denom that the hook caller is registered for
    for denom in hooked_denoms {
        update_rewards(&mut deps, &env, &addr, denom)?;
    }
    Ok(Response::new().add_attribute("action", "stake"))
}

pub(crate) fn execute_unstake(
    mut deps: DepsMut,
    env: Env,
    addr: Addr,
    hooked_denoms: Vec<String>,
) -> Result<Response, ContractError> {
    // update rewards for every denom that the hook caller is registered for
    for denom in hooked_denoms {
        update_rewards(&mut deps, &env, &addr, denom)?;
    }
    Ok(Response::new().add_attribute("action", "unstake"))
}
