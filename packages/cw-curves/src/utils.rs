use cosmwasm_std::Decimal as StdDecimal;
use integer_cbrt::IntegerCubeRoot;
use integer_sqrt::IntegerSquareRoot;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use std::str::FromStr;

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

/// StdDecimal stores as a u128 with 18 decimal points of precision
pub fn decimal_to_std(x: Decimal) -> StdDecimal {
    // this seems straight-forward (if inefficient), converting via string representation
    // TODO: execute errors better? Result?
    StdDecimal::from_str(&x.to_string()).unwrap()

    // // maybe a better approach doing math, not sure about rounding
    //
    // // try to preserve decimal points, max 9
    // let digits = min(x.scale(), 9);
    // let multiplier = 10u128.pow(digits);
    //
    // // we multiply up before we round off to u128,
    // // let StdDecimal do its best to keep these decimal places
    // let nominator = (x * decimal(multiplier, 0)).to_u128().unwrap();
    // StdDecimal::from_ratio(nominator, multiplier)
}

// we multiply by 10^18, turn to int, take square root, then divide by 10^9 as we convert back to decimal
pub(crate) fn square_root(square: Decimal) -> Decimal {
    // must be even
    // TODO: this can overflow easily at 18... what is a good value?
    const EXTRA_DIGITS: u32 = 12;
    let multiplier = 10u128.saturating_pow(EXTRA_DIGITS);

    // multiply by 10^18 and turn to u128
    let extended = square * decimal(multiplier, 0);
    let extended = extended.floor().to_u128().unwrap();

    // take square root, and build a decimal again
    let root = extended.integer_sqrt();
    decimal(root, EXTRA_DIGITS / 2)
}

// we multiply by 10^15, turn to int, take cube root, then divide by 10^5 as we convert back to decimal.
// EXTRA_DIGITS raised from 9 to 15 (audit fix L-6) — earlier 3-decimal precision after cube root
// produced ~0.001 supply-unit dust per sell on SquareRoot curves with >= 4-decimal supply tokens.
pub(crate) fn cube_root(cube: Decimal) -> Decimal {
    // must be multiple of 3
    const EXTRA_DIGITS: u32 = 15;
    let multiplier = 10u128.saturating_pow(EXTRA_DIGITS);

    // multiply out and turn to u128
    let extended = cube * decimal(multiplier, 0);
    let extended = extended.floor().to_u128().unwrap();

    // take cube root, and build a decimal again
    let root = extended.integer_cbrt();
    decimal(root, EXTRA_DIGITS / 3)
}
