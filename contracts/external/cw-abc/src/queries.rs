use crate::abc::CurveFn;
use crate::msg::{
    CommonsPhaseConfigResponse, CurveInfoResponse, DenomResponse, DonationsResponse,
    HatcherAllowlistResponse, HatchersResponse,
};
use crate::state::{
    CurveState, CURVE_STATE, DONATIONS, HATCHERS, HATCHER_ALLOWLIST, INITIAL_SUPPLY, MAX_SUPPLY,
    PHASE, PHASE_CONFIG, SUPPLY_DENOM,
};
use cosmwasm_std::{Deps, Order, QuerierWrapper, StdResult, Uint128};
use std::ops::Deref;

/// Get the current state of the curve
pub fn query_curve_info(deps: Deps, curve_fn: CurveFn) -> StdResult<CurveInfoResponse> {
    let CurveState {
        reserve,
        supply,
        reserve_denom,
        decimals,
        funding,
    } = CURVE_STATE.load(deps.storage)?;

    // This we can get from the local digits stored in instantiate
    let curve = curve_fn(decimals);
    let spot_price = curve.spot_price(supply);

    Ok(CurveInfoResponse {
        reserve,
        supply,
        funding,
        spot_price,
        reserve_denom,
    })
}

/// Returns information about the supply Denom
pub fn get_denom(deps: Deps) -> StdResult<DenomResponse> {
    let denom = SUPPLY_DENOM.load(deps.storage)?;
    Ok(DenomResponse { denom })
}

pub fn query_donations(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<DonationsResponse> {
    let donations = cw_paginate_storage::paginate_map(
        Deps {
            storage: deps.storage,
            api: deps.api,
            querier: QuerierWrapper::new(deps.querier.deref()),
        },
        &DONATIONS,
        start_after
            .map(|addr| deps.api.addr_validate(&addr))
            .transpose()?
            .as_ref(),
        limit,
        Order::Descending,
    )?;

    Ok(DonationsResponse { donations })
}

/// Query hatchers who contributed during the hatch phase
pub fn query_hatchers(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<HatchersResponse> {
    let hatchers = cw_paginate_storage::paginate_map(
        Deps {
            storage: deps.storage,
            api: deps.api,
            querier: QuerierWrapper::new(deps.querier.deref()),
        },
        &HATCHERS,
        start_after
            .map(|addr| deps.api.addr_validate(&addr))
            .transpose()?
            .as_ref(),
        limit,
        Order::Descending,
    )?;

    Ok(HatchersResponse { hatchers })
}

/// Query hatcher allowlist
pub fn query_hatcher_allowlist(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<HatcherAllowlistResponse> {
    if HATCHER_ALLOWLIST.is_empty(deps.storage) {
        return Ok(HatcherAllowlistResponse { allowlist: None });
    }

    let allowlist = cw_paginate_storage::paginate_map(
        Deps {
            storage: deps.storage,
            api: deps.api,
            querier: QuerierWrapper::new(deps.querier.deref()),
        },
        &HATCHER_ALLOWLIST,
        start_after
            .map(|addr| deps.api.addr_validate(&addr))
            .transpose()?
            .as_ref(),
        limit,
        Order::Descending,
    )?;

    Ok(HatcherAllowlistResponse {
        allowlist: Some(allowlist),
    })
}

/// Query the initial supply of the supply token when the ABC was created
pub fn query_initial_supply(deps: Deps) -> StdResult<Uint128> {
    let initial_supply = INITIAL_SUPPLY.may_load(deps.storage)?;
    Ok(initial_supply.unwrap_or_default())
}

/// Query the max supply of the supply token
pub fn query_max_supply(deps: Deps) -> StdResult<Uint128> {
    let max_supply = MAX_SUPPLY.may_load(deps.storage)?;
    Ok(max_supply.unwrap_or(Uint128::MAX))
}

/// Load and return the phase config
pub fn query_phase_config(deps: Deps) -> StdResult<CommonsPhaseConfigResponse> {
    let phase = PHASE.load(deps.storage)?;
    let phase_config = PHASE_CONFIG.load(deps.storage)?;
    Ok(CommonsPhaseConfigResponse {
        phase_config,
        phase,
    })
}
