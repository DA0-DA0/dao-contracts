use cosmwasm_std::{Addr, Deps, Order, StdResult, Uint128};
use cw_storage_plus::{Bound, Map};

use crate::msg::{
    AllowanceInfo, AllowanceResponse, AllowancesResponse, BlacklisteesResponse,
    BlacklisterAllowancesResponse, DenomResponse, FreezerAllowancesResponse, IsFrozenResponse,
    OwnerResponse, StatusInfo, StatusResponse,
};
use crate::state::{
    BLACKLISTED_ADDRESSES, BLACKLISTER_ALLOWANCES, BURNER_ALLOWANCES, DENOM, FREEZER_ALLOWANCES,
    IS_FROZEN, MINTER_ALLOWANCES, OWNER,
};

// Default settings for pagination
const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;

pub fn query_denom(deps: Deps) -> StdResult<DenomResponse> {
    let denom = DENOM.load(deps.storage)?;
    Ok(DenomResponse { denom })
}

pub fn query_is_frozen(deps: Deps) -> StdResult<IsFrozenResponse> {
    let is_frozen = IS_FROZEN.load(deps.storage)?;
    Ok(IsFrozenResponse { is_frozen })
}

pub fn query_owner(deps: Deps) -> StdResult<OwnerResponse> {
    let owner = OWNER.load(deps.storage)?;
    Ok(OwnerResponse {
        address: owner.into_string(),
    })
}

pub fn query_mint_allowance(deps: Deps, address: String) -> StdResult<AllowanceResponse> {
    let allowance = MINTER_ALLOWANCES
        .may_load(deps.storage, &deps.api.addr_validate(&address)?)?
        .unwrap_or_else(Uint128::zero);
    Ok(AllowanceResponse { allowance })
}

pub fn query_burn_allowance(deps: Deps, address: String) -> StdResult<AllowanceResponse> {
    let allowance = BURNER_ALLOWANCES
        .may_load(deps.storage, &deps.api.addr_validate(&address)?)?
        .unwrap_or_else(Uint128::zero);
    Ok(AllowanceResponse { allowance })
}

pub fn query_allowances(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
    allowances: Map<&Addr, Uint128>,
) -> StdResult<Vec<AllowanceInfo>> {
    // based on this query written by larry https://github.com/st4k3h0us3/steak-contracts/blob/854c15c8d1a62303b931a785494a6ecd4b6eaf2a/contracts/hub/src/queries.rs#L90
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let addr: Addr;
    let start = match start_after {
        None => None,
        Some(addr_str) => {
            addr = deps.api.addr_validate(&addr_str)?;
            Some(Bound::exclusive(&addr))
        }
    };

    // this code is based on the code from mars protocol. https://github.com/mars-protocol/fields-of-mars/blob/598af9ff3de7fa9ce65db713a3125fb442ebcf5c/contracts/martian-field/src/queries.rs#L37
    allowances
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (k, v) = item?;
            Ok(AllowanceInfo {
                address: k.to_string(),
                allowance: v,
            })
        })
        .collect()
}

pub fn query_mint_allowances(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<AllowancesResponse> {
    Ok(AllowancesResponse {
        allowances: query_allowances(deps, start_after, limit, MINTER_ALLOWANCES)?,
    })
}

pub fn query_burn_allowances(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<AllowancesResponse> {
    Ok(AllowancesResponse {
        allowances: query_allowances(deps, start_after, limit, BURNER_ALLOWANCES)?,
    })
}

pub fn query_is_blacklisted(deps: Deps, address: String) -> StdResult<StatusResponse> {
    let status = BLACKLISTED_ADDRESSES
        .load(deps.storage, &deps.api.addr_validate(&address)?)
        .unwrap_or(false);
    Ok(StatusResponse { status })
}

pub fn query_status_map(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
    map: Map<&Addr, bool>,
) -> StdResult<Vec<StatusInfo>> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let addr: Addr;
    let start = match start_after {
        None => None,
        Some(addr_str) => {
            addr = deps.api.addr_validate(&addr_str)?;
            Some(Bound::exclusive(&addr))
        }
    };

    map.range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (address, status) = item?;
            Ok(StatusInfo {
                address: address.to_string(),
                status,
            })
        })
        .collect()
}

pub fn query_blacklistees(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<BlacklisteesResponse> {
    Ok(BlacklisteesResponse {
        blacklistees: query_status_map(deps, start_after, limit, BLACKLISTED_ADDRESSES)?,
    })
}

pub fn query_is_blacklister(deps: Deps, address: String) -> StdResult<StatusResponse> {
    let status = BLACKLISTER_ALLOWANCES
        .load(deps.storage, &deps.api.addr_validate(&address)?)
        .unwrap_or(false);
    Ok(StatusResponse { status })
}

pub fn query_blacklister_allowances(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<BlacklisterAllowancesResponse> {
    Ok(BlacklisterAllowancesResponse {
        blacklisters: query_status_map(deps, start_after, limit, BLACKLISTER_ALLOWANCES)?,
    })
}

pub fn query_freezer_allowances(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<FreezerAllowancesResponse> {
    Ok(FreezerAllowancesResponse {
        freezers: query_status_map(deps, start_after, limit, FREEZER_ALLOWANCES)?,
    })
}

pub fn query_is_freezer(deps: Deps, address: String) -> StdResult<StatusResponse> {
    let status = FREEZER_ALLOWANCES
        .load(deps.storage, &deps.api.addr_validate(&address)?)
        .unwrap_or(false);
    Ok(StatusResponse { status })
}

// query inspiration see https://github.com/mars-protocol/fields-of-mars/blob/v1.0.0/packages/fields-of-mars/src/martian_field.rs#L465-L473
