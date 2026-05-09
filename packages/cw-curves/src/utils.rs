use cosmwasm_std::Decimal as StdDecimal;
use integer_cbrt::IntegerCubeRoot;
use integer_sqrt::IntegerSquareRoot;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use std::str::FromStr;

use crate::CurveError;

/// decimal returns an object = num * 10 ^ -scale
/// We use this function in contract.rs rather than call the crate constructor
/// itself, in case we want to swap out the implementation, we can do it only in this file.
///
/// L-4: guards against the silent-corruption footgun where `u128 -> i128 as i128`
/// would wrap to a negative number for inputs >= 2^127. Practically unreachable
/// for any realistic Uint128 supply/reserve, but the panic is preferable to
/// silently returning a wrong-signed Decimal.
pub fn decimal<T: Into<u128>>(num: T, scale: u32) -> Decimal {
    let n: u128 = num.into();
    assert!(
        n <= i128::MAX as u128,
        "cw-curves::decimal overflow: value exceeds i128::MAX"
    );
    Decimal::from_i128_with_scale(n as i128, scale)
}

/// StdDecimal stores as a u128 with 18 decimal points of precision.
///
/// L-5: returns Result so overflow surfaces as a typed error rather than
/// a panic. Conversion through string is rare-path; the error is mostly
/// theoretical for in-range Uint128 values.
pub fn decimal_to_std(x: Decimal) -> Result<StdDecimal, CurveError> {
    StdDecimal::from_str(&x.to_string()).map_err(|_| CurveError::Overflow {
        scale: 0,
        value: x.to_string(),
    })
}

// we multiply by 10^12, turn to int, take square root, then divide by 10^6 as we convert back to decimal.
// L-5: panics on `to_u128` are converted to typed `CurveError::Overflow`.
pub(crate) fn square_root(square: Decimal) -> Result<Decimal, CurveError> {
    // must be even
    const EXTRA_DIGITS: u32 = 12;
    let multiplier = 10u128.saturating_pow(EXTRA_DIGITS);

    let extended = square * decimal(multiplier, 0);
    let extended = extended.floor().to_u128().ok_or(CurveError::Overflow {
        scale: EXTRA_DIGITS,
        value: square.to_string(),
    })?;

    let root = extended.integer_sqrt();
    Ok(decimal(root, EXTRA_DIGITS / 2))
}

// we multiply by 10^15, turn to int, take cube root, then divide by 10^5 as we convert back to decimal.
// EXTRA_DIGITS raised from 9 to 15 (audit fix L-6) — earlier 3-decimal precision after cube root
// produced ~0.001 supply-unit dust per sell on SquareRoot curves with >= 4-decimal supply tokens.
// L-5: panics converted to typed CurveError::Overflow.
pub(crate) fn cube_root(cube: Decimal) -> Result<Decimal, CurveError> {
    // must be multiple of 3
    const EXTRA_DIGITS: u32 = 15;
    let multiplier = 10u128.saturating_pow(EXTRA_DIGITS);

    let extended = cube * decimal(multiplier, 0);
    let extended = extended.floor().to_u128().ok_or(CurveError::Overflow {
        scale: EXTRA_DIGITS,
        value: cube.to_string(),
    })?;

    let root = extended.integer_cbrt();
    Ok(decimal(root, EXTRA_DIGITS / 3))
}
