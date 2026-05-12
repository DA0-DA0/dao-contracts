use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Decimal, Uint128};

use crate::{Curve, CurveError, PiecewiseLinear, SaturatingLinear};

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ScalableCurve {
    Constant { ratio: Decimal },
    ScalableLinear(ScalableLinear),
    ScalablePiecewise(ScalablePiecewise),
}

impl ScalableCurve {
    pub fn scale(self, amount: Uint128) -> Curve {
        match self {
            ScalableCurve::Constant { ratio } => Curve::Constant {
                y: amount.mul_floor(ratio),
            },
            ScalableCurve::ScalableLinear(s) => s.scale(amount),
            ScalableCurve::ScalablePiecewise(p) => p.scale(amount),
        }
    }

    pub fn validate_monotonic_increasing(&self) -> Result<(), CurveError> {
        self.clone()
            .scale(Uint128::new(1_000_000_000))
            .validate_monotonic_increasing()
    }

    pub fn validate_monotonic_decreasing(&self) -> Result<(), CurveError> {
        self.clone()
            .scale(Uint128::new(1_000_000_000))
            .validate_monotonic_decreasing()
    }

    pub fn linear((min_x, min_percent): (u64, u64), (max_x, max_percent): (u64, u64)) -> Self {
        ScalableCurve::ScalableLinear(ScalableLinear {
            min_x,
            min_y: Decimal::percent(min_percent),
            max_x,
            max_y: Decimal::percent(max_percent),
        })
    }
}

/// min_y for all x <= min_x, max_y for all x >= max_x, linear in between
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct ScalableLinear {
    pub min_x: u64,
    pub min_y: Decimal,
    pub max_x: u64,
    pub max_y: Decimal,
}

impl ScalableLinear {
    pub fn scale(self, amount: Uint128) -> Curve {
        Curve::SaturatingLinear(SaturatingLinear {
            min_x: self.min_x,
            min_y: amount.mul_floor(self.min_y),
            max_x: self.max_x,
            max_y: amount.mul_floor(self.max_y),
        })
    }
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct ScalablePiecewise {
    pub steps: Vec<(u64, Decimal)>,
}

impl ScalablePiecewise {
    pub fn scale(self, amount: Uint128) -> Curve {
        let steps = self
            .steps
            .into_iter()
            .map(|(x, y)| (x, amount.mul_floor(y)))
            .collect();
        Curve::PiecewiseLinear(PiecewiseLinear { steps })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn scale_constant() {
        let flex = ScalableCurve::Constant {
            ratio: Decimal::percent(50),
        };
        let curve = flex.scale(Uint128::new(444));
        assert_eq!(curve, Curve::constant(222));
    }

    #[test]
    fn scale_linear() {
        let (min_x, max_x) = (10000, 20000);
        let flex = ScalableCurve::ScalableLinear(ScalableLinear {
            min_x,
            min_y: Decimal::percent(80),
            max_x,
            max_y: Decimal::percent(20),
        });
        let curve = flex.scale(Uint128::new(1100));
        assert_eq!(curve, Curve::saturating_linear((min_x, 880), (max_x, 220)));
    }

    #[test]
    fn scale_piecewise() {
        let (x1, x2, x3) = (10000, 20000, 25000);
        let flex = ScalableCurve::ScalablePiecewise(ScalablePiecewise {
            steps: vec![
                (x1, Decimal::percent(100)),
                (x2, Decimal::percent(70)),
                (x3, Decimal::percent(0)),
            ],
        });
        let curve = flex.scale(Uint128::new(8000));
        assert_eq!(
            curve,
            Curve::PiecewiseLinear(PiecewiseLinear {
                steps: vec![
                    (x1, Uint128::new(8000)),
                    (x2, Uint128::new(5600)),
                    (x3, Uint128::new(0)),
                ]
            })
        );
    }
}
