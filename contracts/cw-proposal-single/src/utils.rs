use cosmwasm_std::{Addr, Deps, StdResult, Uint128};

use cw_core_interface::voting;

pub fn get_voting_power(
    deps: Deps,
    address: Addr,
    dao: Addr,
    height: Option<u64>,
) -> StdResult<Uint128> {
    let response: voting::VotingPowerAtHeightResponse = deps.querier.query_wasm_smart(
        dao,
        &voting::Query::VotingPowerAtHeight {
            address: address.to_string(),
            height,
        },
    )?;
    Ok(response.power)
}

pub fn get_total_power(deps: Deps, dao: Addr, height: Option<u64>) -> StdResult<Uint128> {
    let response: voting::TotalPowerAtHeightResponse = deps
        .querier
        .query_wasm_smart(dao, &voting::Query::TotalPowerAtHeight { height })?;
    Ok(response.power)
}
