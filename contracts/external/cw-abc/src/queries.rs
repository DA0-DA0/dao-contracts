use cosmwasm_std::{Deps, StdResult};
use token_bindings::TokenFactoryQuery;
use crate::abc::CurveFn;
use crate::msg::CurveInfoResponse;
use crate::state::{CURVE_STATE, CurveState};

/// Get the current state of the curve
pub fn query_curve_info(
    deps: Deps<TokenFactoryQuery>,
    curve_fn: CurveFn,
) -> StdResult<CurveInfoResponse> {
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


// // TODO, maybe we don't need this
// pub fn get_denom(
//     deps: Deps<TokenFactoryQuery>,
//     creator_addr: String,
//     subdenom: String,
// ) -> GetDenomResponse {
//     let querier = TokenQuerier::new(&deps.querier);
//     let response = querier.full_denom(creator_addr, subdenom).unwrap();

//     GetDenomResponse {
//         denom: response.denom,
//     }
// }