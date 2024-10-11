use cosmwasm_std::{Addr, Decimal, Deps, StdResult, Uint128};

use dao_interface::voting;

use crate::state::{DAO, DELEGATES};

pub fn is_delegate_registered(deps: Deps, delegate: &Addr, height: u64) -> StdResult<bool> {
    DELEGATES
        .may_load_at_height(deps.storage, delegate.clone(), height)
        .map(|d| d.is_some())
}

pub fn get_voting_power(deps: Deps, addr: &Addr, height: Option<u64>) -> StdResult<Uint128> {
    let dao = DAO.load(deps.storage)?;

    let voting_power: voting::VotingPowerAtHeightResponse = deps.querier.query_wasm_smart(
        &dao,
        &voting::Query::VotingPowerAtHeight {
            address: addr.to_string(),
            height,
        },
    )?;

    Ok(voting_power.power)
}

// TODO: precision factor???
pub fn calculate_delegated_vp(vp: Uint128, percent: Decimal) -> Uint128 {
    if percent.is_zero() {
        return Uint128::zero();
    }

    vp.mul_floor(percent)
}
