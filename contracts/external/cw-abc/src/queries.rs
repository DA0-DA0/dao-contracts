use crate::abc::CurveFn;
use crate::helpers::{calculate_buy_quote, calculate_sell_quote};
use crate::msg::{
    CommonsPhaseConfigResponse, CurveInfoResponse, DenomResponse, DonationsResponse,
    HatcherAllowlistResponse, HatchersResponse, QuoteResponse,
};
use crate::state::{
    hatcher_allowlist, CurveState, HatcherAllowlistConfigType, HatcherAllowlistEntry, CURVE_STATE,
    CURVE_TYPE, DONATIONS, HATCHERS, MAX_SUPPLY, PHASE, PHASE_CONFIG, SUPPLY_DENOM,
};
use cosmwasm_std::{Deps, Order, QuerierWrapper, StdError, StdResult, Uint128};
use cw_storage_plus::Bound;
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

/// Query the contribution of a hatcher
pub fn query_hatcher(deps: Deps, addr: String) -> StdResult<Uint128> {
    let addr = deps.api.addr_validate(&addr)?;

    HATCHERS.load(deps.storage, &addr)
}

/// Query hatcher allowlist
pub fn query_hatcher_allowlist(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
    config_type: Option<HatcherAllowlistConfigType>,
) -> StdResult<HatcherAllowlistResponse> {
    if hatcher_allowlist().is_empty(deps.storage) {
        return Ok(HatcherAllowlistResponse { allowlist: None });
    }

    let binding = start_after
        .map(|x| deps.api.addr_validate(&x))
        .transpose()?;
    let start_after_bound = binding.as_ref().map(Bound::exclusive);

    let iter = match config_type {
        Some(config_type) => hatcher_allowlist()
            .idx
            .config_type
            .prefix(config_type.to_string())
            .range(deps.storage, start_after_bound, None, Order::Ascending),
        None => hatcher_allowlist().range(deps.storage, start_after_bound, None, Order::Ascending),
    }
    .map(|result| result.map(|(addr, config)| HatcherAllowlistEntry { addr, config }));

    let allowlist = match limit {
        Some(limit) => iter
            .take(limit.try_into().unwrap())
            .collect::<StdResult<_>>(),
        None => iter.collect::<StdResult<_>>(),
    }?;

    Ok(HatcherAllowlistResponse {
        allowlist: Some(allowlist),
    })
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

/// Get a buy quote
pub fn query_buy_quote(deps: Deps, payment: Uint128) -> StdResult<QuoteResponse> {
    let curve_type = CURVE_TYPE.load(deps.storage)?;
    let curve_state = CURVE_STATE.load(deps.storage)?;
    let phase_config = PHASE_CONFIG.load(deps.storage)?;
    let phase = PHASE.load(deps.storage)?;

    calculate_buy_quote(payment, &curve_type, &curve_state, &phase, &phase_config)
        .map_err(|e| StdError::generic_err(e.to_string()))
}

/// Get a sell quote
pub fn query_sell_quote(deps: Deps, payment: Uint128) -> StdResult<QuoteResponse> {
    let curve_type = CURVE_TYPE.load(deps.storage)?;
    let curve_state = CURVE_STATE.load(deps.storage)?;
    let phase_config = PHASE_CONFIG.load(deps.storage)?;
    let phase = PHASE.load(deps.storage)?;

    calculate_sell_quote(payment, &curve_type, &curve_state, &phase, &phase_config)
        .map_err(|e| StdError::generic_err(e.to_string()))
}
