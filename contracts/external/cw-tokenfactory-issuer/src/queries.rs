use cosmwasm_std::{Addr, Deps, Order, StdResult, Uint128};
use cw_storage_plus::{Bound, Map};

use crate::msg::{
    AllowanceInfo, AllowanceResponse, AllowancesResponse, AllowlistResponse, DenomResponse,
    DenylistResponse, IsFrozenResponse, StatusInfo, StatusResponse,
};
use crate::state::{
    BeforeSendHookInfo, ALLOWLIST, BEFORE_SEND_HOOK_INFO, BURNER_ALLOWANCES, DENOM, DENYLIST,
    IS_FROZEN, MINTER_ALLOWANCES,
};

// Default settings for pagination
const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;

/// Returns the token denom that this contract is the admin for. Response: DenomResponse
pub fn query_denom(deps: Deps) -> StdResult<DenomResponse> {
    let denom = DENOM.load(deps.storage)?;
    Ok(DenomResponse { denom })
}

/// Returns if token transfer is disabled. Response: IsFrozenResponse
pub fn query_is_frozen(deps: Deps) -> StdResult<IsFrozenResponse> {
    let is_frozen = IS_FROZEN.load(deps.storage)?;
    Ok(IsFrozenResponse { is_frozen })
}

/// Returns the owner of the contract. Response: Ownership
pub fn query_owner(deps: Deps) -> StdResult<cw_ownable::Ownership<::cosmwasm_std::Addr>> {
    cw_ownable::get_ownership(deps.storage)
}

/// Returns the mint allowance of the specified user. Response: AllowanceResponse
pub fn query_mint_allowance(deps: Deps, address: String) -> StdResult<AllowanceResponse> {
    let allowance = MINTER_ALLOWANCES
        .may_load(deps.storage, &deps.api.addr_validate(&address)?)?
        .unwrap_or_else(Uint128::zero);
    Ok(AllowanceResponse { allowance })
}

/// Returns the allowance of the specified address. Response: AllowanceResponse
pub fn query_burn_allowance(deps: Deps, address: String) -> StdResult<AllowanceResponse> {
    let allowance = BURNER_ALLOWANCES
        .may_load(deps.storage, &deps.api.addr_validate(&address)?)?
        .unwrap_or_else(Uint128::zero);
    Ok(AllowanceResponse { allowance })
}

/// Helper function used in allowance list queries.
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

/// Enumerates over all allownances. Response: AllowancesResponse
pub fn query_mint_allowances(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<AllowancesResponse> {
    Ok(AllowancesResponse {
        allowances: query_allowances(deps, start_after, limit, MINTER_ALLOWANCES)?,
    })
}

/// Enumerates over all burn allownances. Response: AllowancesResponse
pub fn query_burn_allowances(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<AllowancesResponse> {
    Ok(AllowancesResponse {
        allowances: query_allowances(deps, start_after, limit, BURNER_ALLOWANCES)?,
    })
}

/// Returns wether the user is on denylist or not. Response: StatusResponse
pub fn query_is_denied(deps: Deps, address: String) -> StdResult<StatusResponse> {
    let status = DENYLIST
        .load(deps.storage, &deps.api.addr_validate(&address)?)
        .unwrap_or(false);
    Ok(StatusResponse { status })
}

/// Returns wether the user is on the allowlist or not. Response: StatusResponse
pub fn query_is_allowed(deps: Deps, address: String) -> StdResult<StatusResponse> {
    let status = ALLOWLIST
        .load(deps.storage, &deps.api.addr_validate(&address)?)
        .unwrap_or(false);
    Ok(StatusResponse { status })
}

/// Returns whether features that require MsgBeforeSendHook are enabled.
/// Most Cosmos chains do not support this feature yet.
pub fn query_before_send_hook_features(deps: Deps) -> StdResult<BeforeSendHookInfo> {
    BEFORE_SEND_HOOK_INFO.load(deps.storage)
}

/// A helper function used in list queries
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

/// Enumerates over all addresses on the allowlist. Response: AllowlistResponse
pub fn query_allowlist(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<AllowlistResponse> {
    Ok(AllowlistResponse {
        allowlist: query_status_map(deps, start_after, limit, ALLOWLIST)?,
    })
}

/// Enumerates over all addresses on the denylist. Response: DenylistResponse
pub fn query_denylist(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<DenylistResponse> {
    Ok(DenylistResponse {
        denylist: query_status_map(deps, start_after, limit, DENYLIST)?,
    })
}
