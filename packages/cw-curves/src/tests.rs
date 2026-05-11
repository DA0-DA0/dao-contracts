// Existing happy-path tests; .unwrap() on Result returns added with the L-5
// trait conversion. Boundary / overflow tests live in `diff_tests.rs`.

use cosmwasm_std::{Decimal as StdDecimal, Uint128};

use crate::{
    curves::{Constant, Linear, Power, Sigmoid, SquareRoot},
    utils::{decimal, taylor_exp},
    Curve, DecimalPlaces,
};
use rust_decimal::Decimal;
use std::str::FromStr;

#[test]
fn constant_curve() {
    // supply is nstep (9), reserve is uatom (6)
    let normalize = DecimalPlaces::new(9, 6);
    let curve = Constant::new(decimal(15u128, 1), normalize);

    // do some sanity checks....
    // spot price is always 1.5 ATOM
    assert_eq!(
        StdDecimal::percent(150),
        curve.spot_price(Uint128::new(123)).unwrap()
    );

    // if we have 30 STEP, we should have 45 ATOM
    let reserve = curve.reserve(Uint128::new(30_000_000_000)).unwrap();
    assert_eq!(Uint128::new(45_000_000), reserve);

    // if we have 36 ATOM, we should have 24 STEP
    let supply = curve.supply(Uint128::new(36_000_000)).unwrap();
    assert_eq!(Uint128::new(24_000_000_000), supply);
}

#[test]
fn linear_curve() {
    // supply is usdt (2), reserve is btc (8)
    let normalize = DecimalPlaces::new(2, 8);
    // slope is 0.1 (eg hits 1.0 after 10btc)
    let curve = Linear::new(decimal(1u128, 1), normalize);

    // do some sanity checks....
    // spot price is 0.1 with 1 USDT supply
    assert_eq!(
        StdDecimal::permille(100),
        curve.spot_price(Uint128::new(100)).unwrap()
    );
    // spot price is 1.7 with 17 USDT supply
    assert_eq!(
        StdDecimal::permille(1700),
        curve.spot_price(Uint128::new(1700)).unwrap()
    );
    // spot price is 0.212 with 2.12 USDT supply
    assert_eq!(
        StdDecimal::permille(212),
        curve.spot_price(Uint128::new(212)).unwrap()
    );

    // if we have 10 USDT, we should have 5 BTC
    let reserve = curve.reserve(Uint128::new(1000)).unwrap();
    assert_eq!(Uint128::new(500_000_000), reserve);
    // if we have 20 USDT, we should have 20 BTC
    let reserve = curve.reserve(Uint128::new(2000)).unwrap();
    assert_eq!(Uint128::new(2_000_000_000), reserve);

    // if we have 1.25 BTC, we should have 5 USDT
    let supply = curve.supply(Uint128::new(125_000_000)).unwrap();
    assert_eq!(Uint128::new(500), supply);
    // test square root rounding
    // if we have 1.11 BTC, we should have 4.7116875957... USDT
    let supply = curve.supply(Uint128::new(111_000_000)).unwrap();
    assert_eq!(Uint128::new(471), supply);
}

#[test]
fn sqrt_curve() {
    // supply is utree (6) reserve is chf (2)
    let normalize = DecimalPlaces::new(6, 2);
    // slope is 0.35 (eg hits 0.35 after 1 chf, 3.5 after 100chf)
    let curve = SquareRoot::new(decimal(35u128, 2), normalize);

    // do some sanity checks....
    // spot price is 0.35 with 1 TREE supply
    assert_eq!(
        StdDecimal::percent(35),
        curve.spot_price(Uint128::new(1_000_000)).unwrap()
    );
    // spot price is 3.5 with 100 TREE supply
    assert_eq!(
        StdDecimal::percent(350),
        curve.spot_price(Uint128::new(100_000_000)).unwrap()
    );
    // spot price should be 23.478713763747788 with 4500 TREE supply (test rounding and reporting here)
    // rounds off around 8-9 sig figs (note diff for last points)
    assert_eq!(
        StdDecimal::from_ratio(2347871365u128, 100_000_000u128),
        curve.spot_price(Uint128::new(4_500_000_000)).unwrap()
    );

    // if we have 1 TREE, we should have 0.2333333333333 CHF
    let reserve = curve.reserve(Uint128::new(1_000_000)).unwrap();
    assert_eq!(Uint128::new(23), reserve);
    // if we have 100 TREE, we should have 233.333333333 CHF
    let reserve = curve.reserve(Uint128::new(100_000_000)).unwrap();
    assert_eq!(Uint128::new(23_333), reserve);
    // test rounding
    // if we have 235 TREE, we should have 840.5790828021146 CHF
    let reserve = curve.reserve(Uint128::new(235_000_000)).unwrap();
    assert_eq!(Uint128::new(84_057), reserve); // round down

    // // if we have 0.23 CHF, we should have 0.990453 TREE (round down)
    // L-6: cube_root EXTRA_DIGITS raised from 9 to 15 — precision improved
    // from 990_000 (~0.045% off) to 990_450 (~0.0003% off). True value
    // 990_453.x.
    let supply = curve.supply(Uint128::new(23)).unwrap();
    assert_eq!(Uint128::new(990_450), supply);
    // if we have 840.58 CHF, we should have 235.000170 TREE (round down)
    // L-6: precision boost — was rounding to 235_000_000 (off by 170 micro-TREE),
    // now matches the comment's stated true value.
    let supply = curve.supply(Uint128::new(84058)).unwrap();
    assert_eq!(Uint128::new(235_000_170), supply);
}

#[test]
fn constant_division_by_zero() {
    let normalize = DecimalPlaces::new(6, 6);
    let curve = Constant::new(decimal(0u128, 0), normalize);
    // L-5: zero `value` produces a typed error rather than a panic.
    assert!(matches!(
        curve.supply(Uint128::new(100)),
        Err(crate::CurveError::DivisionByZero)
    ));
}

#[test]
fn linear_division_by_zero() {
    let normalize = DecimalPlaces::new(6, 6);
    let curve = Linear::new(decimal(0u128, 0), normalize);
    assert!(matches!(
        curve.supply(Uint128::new(100)),
        Err(crate::CurveError::DivisionByZero)
    ));
}

#[test]
fn square_root_division_by_zero() {
    let normalize = DecimalPlaces::new(6, 6);
    let curve = SquareRoot::new(decimal(0u128, 0), normalize);
    assert!(matches!(
        curve.supply(Uint128::new(100)),
        Err(crate::CurveError::DivisionByZero)
    ));
}

// ============================================================
// Phase U: Power curve happy paths
// ============================================================

#[test]
fn power_curve_matches_linear_at_n1() {
    // Power with exponent 1/1 should match Linear's behavior.
    let normalize = DecimalPlaces::new(2, 8);
    let linear = Linear::new(decimal(1u128, 1), normalize);
    let power = Power::new(decimal(1u128, 1), 1, 1, normalize);

    for &supply in &[100u128, 500, 1000, 1700, 5000] {
        let lr = linear.reserve(Uint128::new(supply)).unwrap();
        let pr = power.reserve(Uint128::new(supply)).unwrap();
        assert_eq!(
            lr, pr,
            "linear vs power@n=1 reserve mismatch at supply={}",
            supply
        );
    }
}

#[test]
fn power_curve_matches_square_root_at_n_half() {
    // Power with exponent 1/2 should match SquareRoot's behavior.
    let normalize = DecimalPlaces::new(6, 2);
    let sqrt = SquareRoot::new(decimal(35u128, 2), normalize);
    let power = Power::new(decimal(35u128, 2), 1, 2, normalize);

    for &supply in &[1_000_000u128, 50_000_000, 100_000_000] {
        let sr = sqrt.reserve(Uint128::new(supply)).unwrap();
        let pr = power.reserve(Uint128::new(supply)).unwrap();
        // Allow 1-unit floor difference between the two impls.
        let diff = if sr > pr { sr - pr } else { pr - sr };
        assert!(
            diff.u128() <= 1,
            "sqrt vs power@n=1/2 reserve mismatch at supply={}: sqrt={}, power={}",
            supply,
            sr,
            pr
        );
    }
}

#[test]
fn power_curve_matches_constant_at_n0() {
    // Power with exponent 0/1 reduces to constant slope: f(s) = slope.
    // Integral F(s) = slope * s.
    let normalize = DecimalPlaces::new(9, 6);
    let constant = Constant::new(decimal(15u128, 1), normalize);
    let power = Power::new(decimal(15u128, 1), 0, 1, normalize);

    let cr = constant.reserve(Uint128::new(30_000_000_000)).unwrap();
    let pr = power.reserve(Uint128::new(30_000_000_000)).unwrap();
    assert_eq!(cr, pr);
}

#[test]
fn power_curve_round_trip() {
    // Round-trip identity for an exotic exponent (3/4).
    let normalize = DecimalPlaces::new(6, 6);
    let curve = Power::new(decimal(2u128, 0), 3, 4, normalize);
    for &supply in &[1_000_000u128, 5_000_000, 10_000_000, 50_000_000] {
        let r = curve.reserve(Uint128::new(supply)).unwrap();
        let s_back = curve.supply(r).unwrap().u128();
        let diff = if s_back > supply {
            s_back - supply
        } else {
            supply - s_back
        };
        assert!(
            diff <= 1000,
            "power round-trip drift too high at supply={}: r={}, s_back={}, diff={}",
            supply,
            r,
            s_back,
            diff
        );
    }
}

#[test]
fn power_division_by_zero_on_zero_slope() {
    let normalize = DecimalPlaces::new(6, 6);
    let curve = Power::new(decimal(0u128, 0), 1, 2, normalize);
    assert!(matches!(
        curve.supply(Uint128::new(100)),
        Err(crate::CurveError::DivisionByZero)
    ));
}

// ============================================================
// Phase V: taylor_exp helper
// ============================================================

fn approx_eq(a: Decimal, b: Decimal, tol_decimal_places: u32) -> bool {
    let diff = if a > b { a - b } else { b - a };
    let tol = Decimal::new(1, tol_decimal_places); // 10^(-tol_decimal_places)
    diff < tol
}

#[test]
fn taylor_exp_zero_is_one() {
    assert_eq!(taylor_exp(Decimal::ZERO).unwrap(), Decimal::ONE);
}

#[test]
fn taylor_exp_one_matches_e() {
    let expected = Decimal::from_str("2.7182818284590452353602874713").unwrap();
    let actual = taylor_exp(Decimal::ONE).unwrap();
    assert!(
        approx_eq(actual, expected, 6),
        "taylor_exp(1) = {}, expected ≈ {}",
        actual,
        expected
    );
}

#[test]
fn taylor_exp_negative_is_reciprocal() {
    // e^-1 = 1/e ≈ 0.36787944117144233
    let expected = Decimal::from_str("0.36787944117144232159552377").unwrap();
    let actual = taylor_exp(-Decimal::ONE).unwrap();
    assert!(
        approx_eq(actual, expected, 6),
        "taylor_exp(-1) = {}, expected ≈ {}",
        actual,
        expected
    );
}

#[test]
fn taylor_exp_two_point_five() {
    // e^2.5 ≈ 12.182493960703473
    let expected = Decimal::from_str("12.182493960703473").unwrap();
    let actual = taylor_exp(Decimal::from_str("2.5").unwrap()).unwrap();
    assert!(
        approx_eq(actual, expected, 4),
        "taylor_exp(2.5) = {}, expected ≈ {}",
        actual,
        expected
    );
}

#[test]
fn taylor_exp_rejects_too_large() {
    let res = taylor_exp(Decimal::from(50u32));
    assert!(matches!(res, Err(crate::CurveError::Overflow { .. })));
}

// ============================================================
// Phase V: Sigmoid happy paths
// ============================================================

#[test]
fn sigmoid_spot_price_at_midpoint_is_half_amplitude() {
    // f(midpoint) = a / (1 + e^0) = a / 2.
    let normalize = DecimalPlaces::new(6, 6);
    let curve = Sigmoid::new(
        Decimal::from(2u32), // amplitude = 2
        Decimal::ONE,        // steepness = 1
        Decimal::ONE,        // midpoint = 1.0 (normalized)
        normalize,
    );
    // 1.0 normalized supply = 1_000_000 raw at 6 decimals.
    let actual = curve.spot_price(Uint128::new(1_000_000)).unwrap();
    let expected = cosmwasm_std::Decimal::from_str("1.0").unwrap();
    let diff = if actual > expected {
        actual - expected
    } else {
        expected - actual
    };
    assert!(
        diff < cosmwasm_std::Decimal::from_str("0.001").unwrap(),
        "sigmoid mid-point spot_price = {}, expected ≈ 1.0",
        actual
    );
}

#[test]
fn sigmoid_spot_price_saturates_high_supply() {
    // Far above midpoint, f(s) → amplitude.
    let normalize = DecimalPlaces::new(6, 6);
    let curve = Sigmoid::new(Decimal::from(2u32), Decimal::ONE, Decimal::ONE, normalize);
    // 10.0 normalized supply.
    let actual = curve.spot_price(Uint128::new(10_000_000)).unwrap();
    let expected = cosmwasm_std::Decimal::from_str("2.0").unwrap();
    let diff = if actual > expected {
        actual - expected
    } else {
        expected - actual
    };
    assert!(
        diff < cosmwasm_std::Decimal::from_str("0.01").unwrap(),
        "sigmoid saturated spot_price = {}, expected ≈ 2.0",
        actual
    );
}

#[test]
fn sigmoid_reserve_zero_is_zero() {
    let normalize = DecimalPlaces::new(6, 6);
    let curve = Sigmoid::new(Decimal::from(2u32), Decimal::ONE, Decimal::ONE, normalize);
    assert_eq!(curve.reserve(Uint128::zero()).unwrap(), Uint128::zero());
}

#[test]
fn sigmoid_reserve_monotonic() {
    let normalize = DecimalPlaces::new(6, 6);
    let curve = Sigmoid::new(Decimal::from(2u32), Decimal::ONE, Decimal::ONE, normalize);
    let r1 = curve.reserve(Uint128::new(500_000)).unwrap();
    let r2 = curve.reserve(Uint128::new(1_000_000)).unwrap();
    let r3 = curve.reserve(Uint128::new(2_000_000)).unwrap();
    assert!(r1 < r2);
    assert!(r2 < r3);
}

#[test]
fn sigmoid_round_trip_within_bounds() {
    // Newton-Raphson inverse should land within reasonable tolerance for
    // supply values in the steep-slope region of the curve.
    let normalize = DecimalPlaces::new(6, 6);
    let curve = Sigmoid::new(Decimal::from(2u32), Decimal::ONE, Decimal::ONE, normalize);
    for &s in &[500_000u128, 1_000_000, 1_500_000] {
        let r = curve.reserve(Uint128::new(s)).unwrap();
        let s_back = curve.supply(r).unwrap().u128();
        let diff = if s_back > s { s_back - s } else { s - s_back };
        // Simpson's rule + Newton may drift up to ~1% of full-scale; accept
        // 100k supply units (10% of midpoint) as the tolerance — this is
        // research-quality precision, not production.
        assert!(
            diff <= 100_000,
            "sigmoid round-trip drift too high: s={} -> r={} -> s_back={}",
            s,
            r,
            s_back
        );
    }
}

#[test]
fn sigmoid_division_by_zero_on_zero_amplitude() {
    let normalize = DecimalPlaces::new(6, 6);
    let curve = Sigmoid::new(Decimal::ZERO, Decimal::ONE, Decimal::ONE, normalize);
    assert!(matches!(
        curve.supply(Uint128::new(100)),
        Err(crate::CurveError::DivisionByZero)
    ));
}

// Note: L-5 overflow propagation is exercised in cw-abc's contract tests
// via ContractError::CurveError. Triggering the path purely in cw-curves
// requires bypassing the upstream `decimal()` assert (L-4), which would
// require constructing a Decimal directly — out of scope for this layer.
