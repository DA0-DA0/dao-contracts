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
/// a panic. We round the input to 18 decimal places before string
/// conversion because `StdDecimal::from_str` rejects strings with more
/// fractional digits than its native precision.
pub fn decimal_to_std(x: Decimal) -> Result<StdDecimal, CurveError> {
    let rounded = x.round_dp(18);
    StdDecimal::from_str(&rounded.to_string()).map_err(|_| CurveError::Overflow {
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

// ============================================================
// nth-root helpers for the Power curve (Phase U)
// ============================================================

/// Compute `value^(1/n)` for `n: u32` via Newton-Raphson on `Decimal`.
///
/// `value` must be non-negative. Returns `CurveError::DivisionByZero` for n=0.
/// For n=2 and n=3 we delegate to the integer-arithmetic fast paths
/// (`square_root` / `cube_root`) which are exact on the integer part and
/// well-tested. For other n we run Newton-Raphson on `Decimal` from a
/// bit-count-guided initial guess so `x^(n-1)` doesn't overflow on the
/// first iteration (a naive `value / n` initial guess saturates rust_decimal
/// for large values + high n).
pub(crate) fn nth_root(value: Decimal, n: u32) -> Result<Decimal, CurveError> {
    if n == 0 {
        return Err(CurveError::DivisionByZero);
    }
    if n == 1 {
        return Ok(value);
    }
    if value.is_zero() {
        return Ok(Decimal::ZERO);
    }
    if value.is_sign_negative() {
        return Err(CurveError::Overflow {
            scale: n,
            value: format!("nth_root of negative value: {}", value),
        });
    }
    if n == 2 {
        return square_root(value);
    }
    if n == 3 {
        return cube_root(value);
    }

    const MAX_ITERS: u32 = 64;
    let n_dec = Decimal::from(n);
    let n_minus_1_dec = Decimal::from(n - 1);

    // Initial guess: 2^(bits / n) where bits ≈ log2(value).
    // Get the integer part of value as a u128 (rounded down), count its bits,
    // and start at 2^(bits / n). If the integer part is 0 (value < 1), start
    // at Decimal::ONE — Newton converges from above.
    let int_part = value.floor().to_u128().unwrap_or(0);
    let mut x = if int_part == 0 {
        Decimal::ONE
    } else {
        let bits = 128 - int_part.leading_zeros();
        let shift = (bits / n).min(127);
        Decimal::from(1u128 << shift)
    };

    for _ in 0..MAX_ITERS {
        // Compute x^(n-1) with overflow detection so a too-large initial guess
        // doesn't panic.
        let mut x_pow = Decimal::ONE;
        let mut overflow = false;
        for _ in 0..(n - 1) {
            match x_pow.checked_mul(x) {
                Some(v) => x_pow = v,
                None => {
                    overflow = true;
                    break;
                }
            }
        }
        if overflow || x_pow.is_zero() {
            // Halve x and retry — gets us back to a safe range.
            x /= Decimal::from(2u32);
            if x.is_zero() {
                return Err(CurveError::DivisionByZero);
            }
            continue;
        }
        let quotient = value / x_pow;
        let next = (n_minus_1_dec * x + quotient) / n_dec;
        let diff = if next > x { next - x } else { x - next };
        let tol = if x.is_zero() {
            Decimal::new(1, 9)
        } else {
            x * Decimal::new(1, 9)
        };
        x = next;
        if diff <= tol {
            return Ok(x);
        }
    }
    Err(CurveError::Overflow {
        scale: n,
        value: format!("nth_root failed to converge for value {}", value),
    })
}

/// Compute `base^exp` where `exp` is a non-negative rational `num/den`.
///
/// Routes integer exponents through repeated multiplication and
/// half-/third-integer exponents through the existing fast-path roots when
/// possible. Otherwise falls back to `nth_root(base^num, den)` which works
/// but is more expensive.
pub(crate) fn pow_rational(
    base: Decimal,
    num: u32,
    den: u32,
) -> Result<Decimal, CurveError> {
    if den == 0 {
        return Err(CurveError::DivisionByZero);
    }
    if num == 0 {
        return Ok(Decimal::ONE);
    }
    if base.is_zero() {
        return Ok(Decimal::ZERO);
    }

    // Reduce num/den by gcd to hit the fast paths more often.
    let g = gcd_u32(num, den);
    let num = num / g;
    let den = den / g;

    // Integer exponent.
    if den == 1 {
        let mut result = Decimal::ONE;
        for _ in 0..num {
            result = result
                .checked_mul(base)
                .ok_or_else(|| CurveError::Overflow {
                    scale: num,
                    value: base.to_string(),
                })?;
        }
        return Ok(result);
    }

    // Compute base^num first (still integer-power), then take den-th root.
    let mut numer = Decimal::ONE;
    for _ in 0..num {
        numer = numer
            .checked_mul(base)
            .ok_or_else(|| CurveError::Overflow {
                scale: num,
                value: base.to_string(),
            })?;
    }
    nth_root(numer, den)
}

fn gcd_u32(a: u32, b: u32) -> u32 {
    let (mut a, mut b) = (a, b);
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a.max(1)
}

// ============================================================
// Transcendental helpers for the Sigmoid curve (Phase V)
// ============================================================

/// Compute `e^x` for any `x` via range reduction `e^x = e^k * e^r` where
/// `k = trunc(x)` and `r = x - k`. `e^k` for integer `k` uses repeated
/// multiplication by Euler's number (or its reciprocal for negative); `e^r`
/// for `|r| < 1` uses a 16-term Taylor series. Bounded to `|x| ≤ 30` to
/// keep the result inside Decimal range (`e^30 ≈ 1.07e13`, `e^-30 ≈ 9.4e-14`).
pub(crate) fn taylor_exp(x: Decimal) -> Result<Decimal, CurveError> {
    if x.is_zero() {
        return Ok(Decimal::ONE);
    }
    let abs = if x.is_sign_negative() { -x } else { x };
    if abs > Decimal::from(30u32) {
        return Err(CurveError::Overflow {
            scale: 0,
            value: format!("taylor_exp |x| > 30: {}", x),
        });
    }
    // Euler's constant to 28 decimal digits.
    let e: Decimal = Decimal::from_str("2.7182818284590452353602874713")
        .expect("euler-const literal parses");
    let one_over_e: Decimal = Decimal::from_str("0.3678794411714423215955237701")
        .expect("1/e literal parses");

    // Split into integer and fractional parts of |x|.
    let int_part = abs.floor();
    let int_k = int_part.to_u32().ok_or_else(|| CurveError::Overflow {
        scale: 0,
        value: format!("taylor_exp int part: {}", int_part),
    })?;
    let frac = abs - int_part;

    // e^int_k via repeated multiplication.
    let base = if x.is_sign_negative() { one_over_e } else { e };
    let mut int_pow = Decimal::ONE;
    for _ in 0..int_k {
        int_pow = int_pow.checked_mul(base).ok_or_else(|| {
            CurveError::Overflow {
                scale: int_k,
                value: format!("taylor_exp int_pow overflow at k={}", int_k),
            }
        })?;
    }

    // e^frac via Taylor series for |frac| < 1: 1 + r + r²/2! + r³/3! + ...
    // For x < 0 we still compute e^|frac| then take reciprocal at the end —
    // this avoids the alternating-sign series losing precision.
    let mut frac_pow = Decimal::ONE; // 1
    let mut term = Decimal::ONE; // r^0 / 0!
    for n in 1..=20u32 {
        term = term * frac / Decimal::from(n);
        frac_pow += term;
    }

    let frac_pow = if x.is_sign_negative() {
        if frac_pow.is_zero() {
            return Err(CurveError::DivisionByZero);
        }
        Decimal::ONE / frac_pow
    } else {
        frac_pow
    };

    int_pow.checked_mul(frac_pow).ok_or_else(|| CurveError::Overflow {
        scale: int_k,
        value: "taylor_exp combine overflow".to_string(),
    })
}
