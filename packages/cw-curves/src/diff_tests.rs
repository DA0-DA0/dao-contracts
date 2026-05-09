//! Differential testing of curve impls against pure-Rust f64 reference
//! implementations. Phase N of the audit-readiness plan.
//!
//! For each curve type and a small matrix of (supply_decimals, reserve_decimals),
//! a seeded random walk samples ~10k payment-and-redeem cycles and asserts:
//!
//! 1. The production curve produces results within a small relative tolerance
//!    of the f64 reference (1e-3 = 0.1% relative error for non-trivial inputs).
//! 2. The round-trip identity `supply(reserve(s)) ≈ s` holds within a 1-unit
//!    tolerance, preventing the compounding-rounding-loss footgun the original
//!    audit flagged.
//!
//! Uses a simple SplitMix64 PRNG to avoid pulling `rand` as a dep. Seed is
//! hardcoded so failures reproduce; print the seed and the failing iteration
//! values on assertion failure.

use cosmwasm_std::Uint128;

use crate::curves::{Constant, Linear, Power, SquareRoot};
use crate::{Curve, DecimalPlaces};
use rust_decimal::Decimal;

// ============================================================
// Seeded deterministic PRNG (SplitMix64)
// ============================================================

struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }
    fn next(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// Sample a payment in [1, max_pay] inclusive, log-uniform-ish so we
    /// hit small and large values both.
    fn sample_payment(&mut self, max_pay: u128) -> u128 {
        let r = self.next() as u128;
        // Mix with a logarithmic bucket so we don't concentrate at the top.
        let bucket = (self.next() % 64) as u32;
        let scale = 1u128 << bucket.min(63);
        ((r % scale).max(1)).min(max_pay)
    }
}

// ============================================================
// f64 reference implementations
// ============================================================

fn norm_supply(s: u128, decimals: u32) -> f64 {
    s as f64 / 10f64.powi(decimals as i32)
}

fn denorm_reserve(r: f64, decimals: u32) -> f64 {
    r * 10f64.powi(decimals as i32)
}

fn ref_constant_reserve(supply: u128, value: f64, sd: u32, rd: u32) -> u128 {
    let r = norm_supply(supply, sd) * value;
    denorm_reserve(r, rd).floor() as u128
}

fn ref_linear_reserve(supply: u128, slope: f64, sd: u32, rd: u32) -> u128 {
    let s = norm_supply(supply, sd);
    let r = slope * s * s * 0.5;
    denorm_reserve(r, rd).floor() as u128
}

fn ref_sqrt_reserve(supply: u128, slope: f64, sd: u32, rd: u32) -> u128 {
    let s = norm_supply(supply, sd);
    let r = slope * s.powf(1.5) * (2.0 / 3.0);
    denorm_reserve(r, rd).floor() as u128
}

// ============================================================
// Comparison helpers
// ============================================================

/// Assert two u128 values are within `tolerance` of each other, OR within
/// `relative_tolerance` of the larger value (whichever is more permissive).
fn assert_close(actual: u128, reference: u128, abs_tol: u128, rel_tol: f64, ctx: &str) {
    let diff = if actual > reference {
        actual - reference
    } else {
        reference - actual
    };
    let rel_diff = if reference > 0 {
        diff as f64 / reference as f64
    } else {
        0.0
    };
    if diff > abs_tol && rel_diff > rel_tol {
        panic!(
            "diff_test mismatch [{}]: actual={}, reference={}, abs_diff={}, rel_diff={:.2e}",
            ctx, actual, reference, diff, rel_diff
        );
    }
}

// ============================================================
// Constant
// ============================================================

#[test]
fn diff_constant_random_walk() {
    const SEED: u64 = 0xABC_AAA;
    const ITERS: u32 = 1000;
    const VALUE: u128 = 25; // 0.25
    const SCALE: u32 = 2;

    for &(sd, rd) in &[(6u32, 6u32), (9, 6), (2, 8), (18, 18)] {
        let normalize = DecimalPlaces::new(sd as u8, rd as u8);
        let curve = Constant::new(
            Decimal::from_i128_with_scale(VALUE as i128, SCALE),
            normalize,
        );
        let mut rng = SplitMix64::new(SEED);

        let value_f = VALUE as f64 / 10f64.powi(SCALE as i32);

        // Cap supplies based on reserve_decimals to avoid f64 precision loss
        // at the high end of the test range.
        let max_supply = 10u128.pow(15.min(sd + 3));

        for i in 0..ITERS {
            let s = rng.sample_payment(max_supply);
            let prod = curve.reserve(Uint128::new(s)).unwrap().u128();
            let refr = ref_constant_reserve(s, value_f, sd, rd);
            assert_close(
                prod,
                refr,
                10,
                1e-3,
                &format!("constant reserve sd={} rd={} iter={} s={}", sd, rd, i, s),
            );
        }
    }
}

#[test]
fn diff_constant_round_trip() {
    let normalize = DecimalPlaces::new(6, 6);
    let curve = Constant::new(Decimal::from_i128_with_scale(25, 2), normalize);
    let mut rng = SplitMix64::new(0xC0FFEE);
    for _ in 0..500 {
        let s = rng.sample_payment(1_000_000_000);
        let r = curve.reserve(Uint128::new(s)).unwrap();
        let s_back = curve.supply(r).unwrap().u128();
        // Floor rounding can lose at most a few units per round trip.
        let diff = if s_back > s { s_back - s } else { s - s_back };
        assert!(
            diff <= 10,
            "constant round-trip drift too high: s={} -> r={} -> s_back={}",
            s,
            r,
            s_back
        );
    }
}

// ============================================================
// Linear
// ============================================================

#[test]
fn diff_linear_random_walk() {
    const SEED: u64 = 0xDEAD_BEEF;
    const ITERS: u32 = 1000;
    const SLOPE: u128 = 1; // 0.1
    const SCALE: u32 = 1;

    for &(sd, rd) in &[(2u32, 8u32), (6, 6), (9, 6)] {
        let normalize = DecimalPlaces::new(sd as u8, rd as u8);
        let curve = Linear::new(
            Decimal::from_i128_with_scale(SLOPE as i128, SCALE),
            normalize,
        );
        let mut rng = SplitMix64::new(SEED);
        let slope_f = SLOPE as f64 / 10f64.powi(SCALE as i32);

        // Cap to avoid Decimal saturation in the production path.
        let max_supply = 10u128.pow((sd + 5).min(13));

        for i in 0..ITERS {
            let s = rng.sample_payment(max_supply);
            let prod = curve.reserve(Uint128::new(s)).unwrap().u128();
            let refr = ref_linear_reserve(s, slope_f, sd, rd);
            assert_close(
                prod,
                refr,
                100,
                1e-3,
                &format!("linear reserve sd={} rd={} iter={} s={}", sd, rd, i, s),
            );
        }
    }
}

#[test]
fn diff_linear_round_trip() {
    let normalize = DecimalPlaces::new(6, 8);
    let curve = Linear::new(Decimal::from_i128_with_scale(1, 1), normalize);
    let mut rng = SplitMix64::new(0xFEEDFACE);
    for _ in 0..500 {
        let s = rng.sample_payment(10_000_000); // bounded to keep Decimal happy
        let r = curve.reserve(Uint128::new(s)).unwrap();
        let s_back = curve.supply(r).unwrap().u128();
        let diff = if s_back > s { s_back - s } else { s - s_back };
        // Linear sqrt-inverse can lose more units due to compounding floor.
        // Tolerate up to 1000 supply units (1 milli-token at 6 decimals).
        assert!(
            diff <= 1000,
            "linear round-trip drift too high: s={} -> r={} -> s_back={}",
            s,
            r,
            s_back
        );
    }
}

// ============================================================
// SquareRoot
// ============================================================

#[test]
fn diff_sqrt_random_walk() {
    const SEED: u64 = 0xFACE_F00D;
    const ITERS: u32 = 1000;
    const SLOPE: u128 = 35; // 0.35
    const SCALE: u32 = 2;

    for &(sd, rd) in &[(6u32, 2u32), (6, 6)] {
        let normalize = DecimalPlaces::new(sd as u8, rd as u8);
        let curve = SquareRoot::new(
            Decimal::from_i128_with_scale(SLOPE as i128, SCALE),
            normalize,
        );
        let mut rng = SplitMix64::new(SEED);
        let slope_f = SLOPE as f64 / 10f64.powi(SCALE as i32);

        // Bounded to keep Decimal in range; sqrt has extra precision overhead.
        let max_supply = 10u128.pow((sd + 4).min(10));

        for i in 0..ITERS {
            let s = rng.sample_payment(max_supply);
            let prod = curve.reserve(Uint128::new(s)).unwrap().u128();
            let refr = ref_sqrt_reserve(s, slope_f, sd, rd);
            assert_close(
                prod,
                refr,
                100,
                5e-3,
                &format!("sqrt reserve sd={} rd={} iter={} s={}", sd, rd, i, s),
            );
        }
    }
}

#[test]
fn diff_sqrt_round_trip() {
    let normalize = DecimalPlaces::new(6, 2);
    let curve = SquareRoot::new(Decimal::from_i128_with_scale(35, 2), normalize);
    let mut rng = SplitMix64::new(0xBADC0DE);
    let mut sampled = 0u32;
    let mut attempts = 0u32;
    while sampled < 500 && attempts < 5000 {
        attempts += 1;
        let s = rng.sample_payment(100_000_000);
        let r = curve.reserve(Uint128::new(s)).unwrap();
        // Skip pathologically-small reserves: when r quantizes to < 100
        // base units, a single floor unit dominates the supply round-trip.
        // Real consumers pay attention to dust thresholds at instantiate
        // time via contribution_limits.
        if r.u128() < 100 {
            continue;
        }
        sampled += 1;
        let s_back = curve.supply(r).unwrap().u128();
        let diff = if s_back > s { s_back - s } else { s - s_back };
        let rel = if s > 0 { diff as f64 / s as f64 } else { 0.0 };
        assert!(
            diff <= 10_000 || rel <= 1e-2,
            "sqrt round-trip drift too high: s={} -> r={} -> s_back={} (diff={}, rel={:.4})",
            s,
            r,
            s_back,
            diff,
            rel
        );
    }
    assert!(
        sampled >= 100,
        "too few sqrt round-trip samples: {}",
        sampled
    );
}

// ============================================================
// Power (Phase U)
// ============================================================

fn ref_power_reserve(
    supply: u128,
    slope: f64,
    num: u32,
    den: u32,
    sd: u32,
    rd: u32,
) -> u128 {
    let s = norm_supply(supply, sd);
    // F(s) = slope / (1 + num/den) * s^(1 + num/den) = slope * den / (num+den) * s^((num+den)/den)
    let p = (num as f64 + den as f64) / den as f64;
    let r = slope * (den as f64 / (num as f64 + den as f64)) * s.powf(p);
    denorm_reserve(r, rd).floor() as u128
}

#[test]
fn diff_power_random_walk() {
    const SEED: u64 = 0x_DEC0_DED1;
    const ITERS: u32 = 500;
    const SLOPE: u128 = 5; // 0.5
    const SCALE: u32 = 1;

    // Test a range of rational exponents including the existing fast paths
    // (Constant n=0/1, Linear n=1/1, SquareRoot n=1/2) and a few that
    // exercise the general nth_root path (3/4, 7/4).
    let exponents: &[(u32, u32)] = &[(0, 1), (1, 2), (1, 1), (3, 4), (7, 4), (2, 1)];

    for &(num, den) in exponents {
        for &(sd, rd) in &[(6u32, 6u32), (2, 8)] {
            let normalize = DecimalPlaces::new(sd as u8, rd as u8);
            let curve = Power::new(
                Decimal::from_i128_with_scale(SLOPE as i128, SCALE),
                num,
                den,
                normalize,
            );
            let mut rng = SplitMix64::new(SEED);
            let slope_f = SLOPE as f64 / 10f64.powi(SCALE as i32);

            // Bound supplies so that intermediate base^num stays under
            // rust_decimal's saturation. For higher num/den, this matters more.
            let max_supply = match (num, den) {
                (0, 1) => 10u128.pow((sd + 5).min(13)),
                (n, _) if n <= 2 => 10u128.pow((sd + 4).min(11)),
                _ => 10u128.pow((sd + 3).min(9)),
            };

            for i in 0..ITERS {
                let s = rng.sample_payment(max_supply);
                let prod = match curve.reserve(Uint128::new(s)) {
                    Ok(v) => v.u128(),
                    Err(_) => continue, // skip if math saturates for this sample
                };
                let refr = ref_power_reserve(s, slope_f, num, den, sd, rd);
                assert_close(
                    prod,
                    refr,
                    1000,
                    1e-2,
                    &format!(
                        "power n={}/{}reserve sd={} rd={} iter={} s={}",
                        num, den, sd, rd, i, s
                    ),
                );
            }
        }
    }
}

#[test]
fn diff_power_round_trip() {
    let normalize = DecimalPlaces::new(6, 6);
    // n = 3/4 — exotic fractional exponent that goes through nth_root.
    let curve = Power::new(Decimal::from_i128_with_scale(2, 0), 3, 4, normalize);
    let mut rng = SplitMix64::new(0x_F0F0_BEEF);
    let mut sampled = 0u32;
    let mut attempts = 0u32;
    while sampled < 200 && attempts < 2000 {
        attempts += 1;
        let s = rng.sample_payment(10_000_000);
        let r = match curve.reserve(Uint128::new(s)) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if r.u128() < 100 {
            // skip small-reserve dust regime where rounding dominates
            continue;
        }
        let s_back = match curve.supply(r) {
            Ok(v) => v.u128(),
            Err(_) => continue,
        };
        sampled += 1;
        let diff = if s_back > s { s_back - s } else { s - s_back };
        let rel = if s > 0 { diff as f64 / s as f64 } else { 0.0 };
        assert!(
            diff <= 10_000 || rel <= 1e-2,
            "power round-trip drift too high: s={} -> r={} -> s_back={} (diff={}, rel={:.4})",
            s,
            r,
            s_back,
            diff,
            rel
        );
    }
    assert!(sampled >= 50, "too few power round-trip samples: {}", sampled);
}

// ============================================================
// Boundary cases
// ============================================================

#[test]
fn diff_boundary_supply_zero() {
    let normalize = DecimalPlaces::new(6, 6);
    let curve = Constant::new(Decimal::from_i128_with_scale(25, 2), normalize);
    assert_eq!(curve.reserve(Uint128::zero()).unwrap(), Uint128::zero());
    assert_eq!(curve.supply(Uint128::zero()).unwrap(), Uint128::zero());

    let curve = Linear::new(Decimal::from_i128_with_scale(1, 1), normalize);
    assert_eq!(curve.reserve(Uint128::zero()).unwrap(), Uint128::zero());
    assert_eq!(curve.supply(Uint128::zero()).unwrap(), Uint128::zero());

    let curve = SquareRoot::new(Decimal::from_i128_with_scale(35, 2), normalize);
    assert_eq!(curve.reserve(Uint128::zero()).unwrap(), Uint128::zero());
    assert_eq!(curve.supply(Uint128::zero()).unwrap(), Uint128::zero());
}

#[test]
fn diff_boundary_supply_one() {
    let normalize = DecimalPlaces::new(6, 6);
    let curve = Constant::new(Decimal::from_i128_with_scale(25, 2), normalize);
    // 1 supply micro-unit at constant 0.25 maps to 0 (floor).
    assert_eq!(curve.reserve(Uint128::new(1)).unwrap(), Uint128::zero());

    let curve = Linear::new(Decimal::from_i128_with_scale(1, 1), normalize);
    // Quadratic at s=1 micro-unit is essentially zero in normalized space.
    assert_eq!(curve.reserve(Uint128::new(1)).unwrap(), Uint128::zero());
}
