use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use thiserror::Error;

use cosmwasm_std::Uint128;

#[derive(Error, Debug, PartialEq)]
pub enum CurveError {
    #[error("Curve isn't monotonic")]
    NotMonotonic,

    #[error("Curve is monotonic increasing")]
    MonotonicIncreasing,

    #[error("Curve is monotonic decreasing")]
    MonotonicDecreasing,

    #[error("Later point must have higher X than previous point")]
    PointsOutOfOrder,

    #[error("No steps defined")]
    MissingSteps,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Curve {
    Constant { y: Uint128 },
    SaturatingLinear(SaturatingLinear),
    PiecewiseLinear(PiecewiseLinear),
}

impl Curve {
    pub fn saturating_linear((min_x, min_y): (u64, u128), (max_x, max_y): (u64, u128)) -> Self {
        Curve::SaturatingLinear(SaturatingLinear {
            min_x,
            min_y: min_y.into(),
            max_x,
            max_y: max_y.into(),
        })
    }

    pub fn constant(y: u128) -> Self {
        Curve::Constant { y: Uint128::new(y) }
    }
}

impl Curve {
    /// provides y = f(x) evaluation
    pub fn value(&self, x: u64) -> Uint128 {
        match self {
            Curve::Constant { y } => *y,
            Curve::SaturatingLinear(s) => s.value(x),
            Curve::PiecewiseLinear(p) => p.value(x),
        }
    }

    /// general sanity checks on input values to ensure this is valid.
    /// these checks should be included by the other validate_* functions
    pub fn validate(&self) -> Result<(), CurveError> {
        match self {
            Curve::Constant { .. } => Ok(()),
            Curve::SaturatingLinear(s) => s.validate(),
            Curve::PiecewiseLinear(p) => p.validate(),
        }
    }

    /// returns an error if there is ever x2 > x1 such that value(x2) < value(x1)
    pub fn validate_monotonic_increasing(&self) -> Result<(), CurveError> {
        match self {
            Curve::Constant { .. } => Ok(()),
            Curve::SaturatingLinear(s) => s.validate_monotonic_increasing(),
            Curve::PiecewiseLinear(p) => p.validate_monotonic_increasing(),
        }
    }

    /// returns an error if there is ever x2 > x1 such that value(x1) < value(x2)
    pub fn validate_monotonic_decreasing(&self) -> Result<(), CurveError> {
        match self {
            Curve::Constant { .. } => Ok(()),
            Curve::SaturatingLinear(s) => s.validate_monotonic_decreasing(),
            Curve::PiecewiseLinear(p) => p.validate_monotonic_decreasing(),
        }
    }

    /// return (min, max) that can ever be returned from value. These could potentially be u128::MIN and u128::MAX
    pub fn range(&self) -> (u128, u128) {
        match self {
            Curve::Constant { y } => (y.u128(), y.u128()),
            Curve::SaturatingLinear(sat) => sat.range(),
            Curve::PiecewiseLinear(p) => p.range(),
        }
    }
}

/// min_y for all x <= min_x, max_y for all x >= max_x, linear in between
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct SaturatingLinear {
    pub min_x: u64,
    // I would use Uint128, but those cause parse error, which was fixed in https://github.com/CosmWasm/serde-json-wasm/pull/37
    // but not yet released in serde-wasm-json v0.4.0
    pub min_y: Uint128,
    pub max_x: u64,
    pub max_y: Uint128,
}

impl SaturatingLinear {
    /// provides y = f(x) evaluation
    pub fn value(&self, x: u64) -> Uint128 {
        match (x < self.min_x, x > self.max_x) {
            (true, _) => self.min_y,
            (_, true) => self.max_y,
            _ => interpolate((self.min_x, self.min_y), (self.max_x, self.max_y), x),
        }
    }

    /// general sanity checks on input values to ensure this is valid.
    /// these checks should be included by the other validate_* functions
    pub fn validate(&self) -> Result<(), CurveError> {
        if self.max_x <= self.min_x {
            return Err(CurveError::PointsOutOfOrder);
        }
        Ok(())
    }

    /// returns an error if there is ever x2 > x1 such that value(x2) < value(x1)
    pub fn validate_monotonic_increasing(&self) -> Result<(), CurveError> {
        self.validate()?;
        if self.max_y < self.min_y {
            return Err(CurveError::MonotonicDecreasing);
        }
        Ok(())
    }

    /// returns an error if there is ever x2 > x1 such that value(x1) < value(x2)
    pub fn validate_monotonic_decreasing(&self) -> Result<(), CurveError> {
        self.validate()?;
        if self.max_y > self.min_y {
            return Err(CurveError::MonotonicIncreasing);
        }
        Ok(())
    }

    /// return (min, max) that can ever be returned from value. These could potentially be 0 and u64::MAX
    pub fn range(&self) -> (u128, u128) {
        if self.max_y > self.min_y {
            (self.min_y.u128(), self.max_y.u128())
        } else {
            (self.max_y.u128(), self.min_y.u128())
        }
    }
}

// this requires min_x < x < max_x to have been previously validated
fn interpolate((min_x, min_y): (u64, Uint128), (max_x, max_y): (u64, Uint128), x: u64) -> Uint128 {
    if max_y > min_y {
        min_y + (max_y - min_y) * Uint128::from(x - min_x) / Uint128::from(max_x - min_x)
    } else {
        min_y - (min_y - max_y) * Uint128::from(x - min_x) / Uint128::from(max_x - min_x)
    }
}

/// This is a generalization of SaturatingLinear, steps must be arranged with increasing time (u64).
/// Any point before first step gets the first value, after last step the last value.
/// Otherwise, it is a linear interpolation between the two closest points.
/// Vec of length 1 -> Constant
/// Vec of length 2 -> SaturatingLinear
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct PiecewiseLinear {
    pub steps: Vec<(u64, Uint128)>,
}

impl PiecewiseLinear {
    /// provides y = f(x) evaluation
    pub fn value(&self, x: u64) -> Uint128 {
        // figure out the pair of points it lies between
        let (mut prev, mut next): (Option<&(u64, Uint128)>, _) = (None, &self.steps[0]);
        for step in &self.steps[1..] {
            // only break if x is not above prev
            if x >= next.0 {
                prev = Some(next);
                next = step;
            } else {
                break;
            }
        }
        // at this time:
        // prev may be None (this was lower than first point)
        // x may equal prev.0 (use this value)
        // x may be greater than next (if higher than last item)
        // OR x may be between prev and next (interpolate)
        if let Some(last) = prev {
            if x == last.0 {
                // this handles exact match with low end
                last.1
            } else if x >= next.0 {
                // this handles both higher than all and exact match
                next.1
            } else {
                // here we do linear interpolation
                interpolate(*last, *next, x)
            }
        } else {
            // lower than all, use first
            next.1
        }
    }

    /// general sanity checks on input values to ensure this is valid.
    /// these checks should be included by the other validate_* functions
    pub fn validate(&self) -> Result<(), CurveError> {
        if self.steps.is_empty() {
            return Err(CurveError::MissingSteps);
        }
        self.steps.iter().fold(Ok(0u64), |acc, (x, _)| {
            acc.and_then(|last| {
                if *x > last {
                    Ok(*x)
                } else {
                    Err(CurveError::PointsOutOfOrder)
                }
            })
        })?;
        Ok(())
    }

    /// returns an error if there is ever x2 > x1 such that value(x2) < value(x1)
    pub fn validate_monotonic_increasing(&self) -> Result<(), CurveError> {
        self.validate()?;
        match self.classify_curve() {
            Shape::NotMonotonic => Err(CurveError::NotMonotonic),
            Shape::MonotonicDecreasing => Err(CurveError::MonotonicDecreasing),
            _ => Ok(()),
        }
    }

    /// returns an error if there is ever x2 > x1 such that value(x1) < value(x2)
    pub fn validate_monotonic_decreasing(&self) -> Result<(), CurveError> {
        self.validate()?;
        match self.classify_curve() {
            Shape::NotMonotonic => Err(CurveError::NotMonotonic),
            Shape::MonotonicIncreasing => Err(CurveError::MonotonicIncreasing),
            _ => Ok(()),
        }
    }

    // Gives monotonic info. Requires there be at least one item in steps
    fn classify_curve(&self) -> Shape {
        let mut iter = self.steps.iter();
        let (_, first) = iter.next().unwrap();
        let (_, shape) = iter.fold((*first, Shape::Constant), |(last, shape), (_, y)| {
            let shape = match (shape, y.cmp(&last)) {
                (Shape::NotMonotonic, _) => Shape::NotMonotonic,
                (Shape::MonotonicDecreasing, Ordering::Greater) => Shape::NotMonotonic,
                (Shape::MonotonicDecreasing, _) => Shape::MonotonicDecreasing,
                (Shape::MonotonicIncreasing, Ordering::Less) => Shape::NotMonotonic,
                (Shape::MonotonicIncreasing, _) => Shape::MonotonicIncreasing,
                (Shape::Constant, Ordering::Greater) => Shape::MonotonicIncreasing,
                (Shape::Constant, Ordering::Less) => Shape::MonotonicDecreasing,
                (Shape::Constant, Ordering::Equal) => Shape::Constant,
            };
            (*y, shape)
        });
        shape
    }

    /// return (min, max) that can ever be returned from value. These could potentially be 0 and u64::MAX
    pub fn range(&self) -> (u128, u128) {
        let low = self.steps.iter().map(|(_, y)| *y).min().unwrap().u128();
        let high = self.steps.iter().map(|(_, y)| *y).max().unwrap().u128();
        (low, high)
    }
}

enum Shape {
    // If there is only one point, or all have same value
    Constant,
    MonotonicIncreasing,
    MonotonicDecreasing,
    NotMonotonic,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constant() {
        let y = 524;
        let curve = Curve::constant(y);

        // always valid
        curve.validate().unwrap();
        curve.validate_monotonic_increasing().unwrap();
        curve.validate_monotonic_decreasing().unwrap();

        // always returns same value
        assert_eq!(curve.value(1).u128(), y);
        assert_eq!(curve.value(1000000).u128(), y);

        // range is constant
        assert_eq!(curve.range(), (y, y));
    }

    #[test]
    fn test_increasing_linear() {
        let low = (100, 0);
        let high = (200, 50);
        let curve = Curve::saturating_linear(low, high);

        // validly increasing
        curve.validate().unwrap();
        curve.validate_monotonic_increasing().unwrap();
        // but not decreasing
        let err = curve.validate_monotonic_decreasing().unwrap_err();
        assert_eq!(err, CurveError::MonotonicIncreasing);

        // check extremes
        assert_eq!(curve.value(1).u128(), low.1);
        assert_eq!(curve.value(1000000).u128(), high.1);
        // check linear portion
        assert_eq!(curve.value(150).u128(), 25);
        // and rounding
        assert_eq!(curve.value(103).u128(), 1);

        // range is min to max
        assert_eq!(curve.range(), (low.1, high.1));
    }

    #[test]
    fn test_decreasing_linear() {
        let low = (1700, 500);
        let high = (2000, 200);
        let curve = Curve::saturating_linear(low, high);

        // validly decreasing
        curve.validate().unwrap();
        curve.validate_monotonic_decreasing().unwrap();
        // but not increasing
        let err = curve.validate_monotonic_increasing().unwrap_err();
        assert_eq!(err, CurveError::MonotonicDecreasing);

        // check extremes
        assert_eq!(curve.value(low.0 - 5).u128(), low.1);
        assert_eq!(curve.value(high.0 + 5).u128(), high.1);
        // check linear portion
        assert_eq!(curve.value(1800).u128(), 400);
        assert_eq!(curve.value(1997).u128(), 203);

        // range is min to max
        assert_eq!(curve.range(), (high.1, low.1));
    }

    #[test]
    fn test_invalid_linear() {
        let low = (15000, 100);
        let high = (12000, 200);
        let curve = Curve::saturating_linear(low, high);

        // always invalid
        let err = curve.validate().unwrap_err();
        assert_eq!(CurveError::PointsOutOfOrder, err);
        let err = curve.validate_monotonic_decreasing().unwrap_err();
        assert_eq!(CurveError::PointsOutOfOrder, err);
        let err = curve.validate_monotonic_increasing().unwrap_err();
        assert_eq!(CurveError::PointsOutOfOrder, err);
    }

    #[test]
    fn test_piecewise_one_step() {
        let y = 524;
        let curve = Curve::PiecewiseLinear(PiecewiseLinear {
            steps: vec![(12345, Uint128::new(y))],
        });

        // always valid
        curve.validate().unwrap();
        curve.validate_monotonic_increasing().unwrap();
        curve.validate_monotonic_decreasing().unwrap();

        // always returns same value
        assert_eq!(curve.value(1).u128(), y);
        assert_eq!(curve.value(1000000).u128(), y);

        // range is constant
        assert_eq!(curve.range(), (y, y));
    }

    #[test]
    fn test_piecewise_two_point_increasing() {
        let low = (100, Uint128::new(0));
        let high = (200, Uint128::new(50));
        let curve = Curve::PiecewiseLinear(PiecewiseLinear {
            steps: vec![low, high],
        });

        // validly increasing
        curve.validate().unwrap();
        curve.validate_monotonic_increasing().unwrap();
        // but not decreasing
        let err = curve.validate_monotonic_decreasing().unwrap_err();
        assert_eq!(err, CurveError::MonotonicIncreasing);

        // check extremes
        assert_eq!(curve.value(1), low.1);
        assert_eq!(curve.value(1000000), high.1);
        // check linear portion
        assert_eq!(curve.value(150).u128(), 25);
        // and rounding
        assert_eq!(curve.value(103).u128(), 1);
        // check both edges
        assert_eq!(curve.value(low.0), low.1);
        assert_eq!(curve.value(high.0), high.1);

        // range is min to max
        assert_eq!(curve.range(), (low.1.u128(), high.1.u128()));
    }

    #[test]
    fn test_piecewise_two_point_decreasing() {
        let low = (1700, Uint128::new(500));
        let high = (2000, Uint128::new(200));
        let curve = Curve::PiecewiseLinear(PiecewiseLinear {
            steps: vec![low, high],
        });

        // validly decreasing
        curve.validate().unwrap();
        curve.validate_monotonic_decreasing().unwrap();
        // but not increasing
        let err = curve.validate_monotonic_increasing().unwrap_err();
        assert_eq!(err, CurveError::MonotonicDecreasing);

        // check extremes
        assert_eq!(curve.value(low.0 - 5), low.1);
        assert_eq!(curve.value(high.0 + 5), high.1);
        // check linear portion
        assert_eq!(curve.value(1800).u128(), 400);
        assert_eq!(curve.value(1997).u128(), 203);
        // check edge matches
        assert_eq!(curve.value(low.0), low.1);
        assert_eq!(curve.value(high.0), high.1);

        // range is min to max
        assert_eq!(curve.range(), (high.1.u128(), low.1.u128()));
    }

    #[test]
    fn test_piecewise_two_point_invalid() {
        let low = (15000, 100);
        let high = (12000, 200);
        let curve = Curve::saturating_linear(low, high);

        // always invalid
        let err = curve.validate().unwrap_err();
        assert_eq!(CurveError::PointsOutOfOrder, err);
        let err = curve.validate_monotonic_decreasing().unwrap_err();
        assert_eq!(CurveError::PointsOutOfOrder, err);
        let err = curve.validate_monotonic_increasing().unwrap_err();
        assert_eq!(CurveError::PointsOutOfOrder, err);
    }

    #[test]
    fn test_piecewise_three_point_increasing() {
        let low = (100, Uint128::new(0));
        let mid = (200, Uint128::new(100));
        let high = (300, Uint128::new(400));
        let curve = Curve::PiecewiseLinear(PiecewiseLinear {
            steps: vec![low, mid, high],
        });

        // validly increasing
        curve.validate().unwrap();
        curve.validate_monotonic_increasing().unwrap();
        // but not decreasing
        let err = curve.validate_monotonic_decreasing().unwrap_err();
        assert_eq!(err, CurveError::MonotonicIncreasing);

        // check extremes
        assert_eq!(curve.value(1), low.1);
        assert_eq!(curve.value(1000000), high.1);

        // check first portion
        assert_eq!(curve.value(172).u128(), 72);
        // check second portion (100 + 3 * 40) = 220
        assert_eq!(curve.value(240).u128(), 220);

        // check all exact matches
        assert_eq!(curve.value(low.0), low.1);
        assert_eq!(curve.value(mid.0), mid.1);
        assert_eq!(curve.value(high.0), high.1);

        // range is min to max
        assert_eq!(curve.range(), (low.1.u128(), high.1.u128()));
    }

    #[test]
    fn test_piecewise_three_point_decreasing() {
        let low = (100, Uint128::new(400));
        let mid = (200, Uint128::new(100));
        let high = (300, Uint128::new(0));
        let curve = Curve::PiecewiseLinear(PiecewiseLinear {
            steps: vec![low, mid, high],
        });

        // validly decreasing
        curve.validate().unwrap();
        curve.validate_monotonic_decreasing().unwrap();
        // but not increasing
        let err = curve.validate_monotonic_increasing().unwrap_err();
        assert_eq!(err, CurveError::MonotonicDecreasing);

        // check extremes
        assert_eq!(curve.value(1), low.1);
        assert_eq!(curve.value(1000000), high.1);

        // check first portion (400 - 72 * 3 = 184)
        assert_eq!(curve.value(172).u128(), 184);
        // check second portion (100 + 45) = 55
        assert_eq!(curve.value(245).u128(), 55);

        // check all exact matches
        assert_eq!(curve.value(low.0), low.1);
        assert_eq!(curve.value(mid.0), mid.1);
        assert_eq!(curve.value(high.0), high.1);

        // range is min to max
        assert_eq!(curve.range(), (high.1.u128(), low.1.u128()));
    }

    #[test]
    fn test_piecewise_three_point_invalid_not_monotonic() {
        let low = (100, Uint128::new(400));
        let mid = (200, Uint128::new(100));
        let high = (300, Uint128::new(300));
        let curve = Curve::PiecewiseLinear(PiecewiseLinear {
            steps: vec![low, mid, high],
        });

        // validly order
        curve.validate().unwrap();
        // not monotonic
        let err = curve.validate_monotonic_increasing().unwrap_err();
        assert_eq!(err, CurveError::NotMonotonic);
        // not increasing
        let err = curve.validate_monotonic_decreasing().unwrap_err();
        assert_eq!(err, CurveError::NotMonotonic);
    }

    #[test]
    fn test_piecewise_three_point_invalid_out_of_order() {
        let low = (100, Uint128::new(400));
        let mid = (200, Uint128::new(100));
        let high = (300, Uint128::new(300));
        let curve = Curve::PiecewiseLinear(PiecewiseLinear {
            steps: vec![low, high, mid],
        });

        // validly order
        let err = curve.validate().unwrap_err();
        assert_eq!(err, CurveError::PointsOutOfOrder);
        // not monotonic
        let err = curve.validate_monotonic_increasing().unwrap_err();
        assert_eq!(err, CurveError::PointsOutOfOrder);
        // not increasing
        let err = curve.validate_monotonic_decreasing().unwrap_err();
        assert_eq!(err, CurveError::PointsOutOfOrder);
    }

    // TODO: multi-step bad
}
