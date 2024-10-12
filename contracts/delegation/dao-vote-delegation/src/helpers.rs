use cosmwasm_std::{Addr, Decimal, Deps, DepsMut, StdResult, Uint128};

use dao_interface::voting;

use crate::{
    state::{
        DAO, DELEGATED_VP, DELEGATES, PROPOSAL_HOOK_CALLERS, UNVOTED_DELEGATED_VP,
        VOTING_POWER_HOOK_CALLERS,
    },
    ContractError,
};

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

    let voting_power: voting::VotingPowerAtHeightResponse = deps.querier.query_wasm_smart(
        &dao,
        &voting::Query::VotingPowerAtHeight {
            address: addr.to_string(),
            height: Some(height),
        },
    )?;

    Ok(voting_power.power)
}

pub fn calculate_delegated_vp(vp: Uint128, percent: Decimal) -> Uint128 {
    if percent.is_zero() || vp.is_zero() {
        return Uint128::zero();
    }

    vp.mul_floor(percent)
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
    height: u64,
) -> StdResult<Uint128> {
    // if no unvoted delegated VP exists for the proposal, use the delegate's
    // total delegated VP at that height. UNVOTED_DELEGATED_VP gets set when the
    // delegate or one of their delegators casts a vote. if empty, none of them
    // have voted yet.
    match UNVOTED_DELEGATED_VP.may_load(deps.storage, (&delegate, &proposal_module, proposal_id))? {
        Some(vp) => Ok(vp),
        None => Ok(DELEGATED_VP
            .may_load_at_height(deps.storage, &delegate, height)?
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
