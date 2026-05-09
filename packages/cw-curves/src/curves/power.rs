use cosmwasm_std::{Decimal as StdDecimal, Uint128};
use rust_decimal::Decimal;

use crate::{
    utils::{decimal_to_std, pow_rational},
    Curve, CurveError, DecimalPlaces,
};

/// `f(s) = slope * s^(num/den)`. Generalizes `Constant` (n=0/1), `Linear`
/// (n=1/1), and `SquareRoot` (n=1/2) under a single curve with a rational
/// exponent. Phase U addition.
///
/// Integral (the `reserve` function): `F(s) = slope * s^((num+den)/den) /
/// ((num + den) / den) = slope * den / (num + den) * s^((num+den)/den)`.
///
/// Inverse (the `supply` function): `F^-1(r) = ((num + den) * r / (slope *
/// den))^(den / (num + den))`.
pub struct Power {
    pub slope: Decimal,
    pub exponent_num: u32,
    pub exponent_den: u32,
    pub normalize: DecimalPlaces,
}

impl Power {
    pub fn new(
        slope: Decimal,
        exponent_num: u32,
        exponent_den: u32,
        normalize: DecimalPlaces,
    ) -> Self {
        Self {
            slope,
            exponent_num,
            exponent_den,
            normalize,
        }
    }

    /// (num + den) / den, used in integral and inverse.
    fn integral_num(&self) -> u32 {
        self.exponent_num + self.exponent_den
    }
}

impl Curve for Power {
    fn spot_price(&self, supply: Uint128) -> Result<StdDecimal, CurveError> {
        // f(x) = slope * supply^(num/den)
        let s = self.normalize.from_supply(supply);
        let powered = pow_rational(s, self.exponent_num, self.exponent_den)?;
        decimal_to_std(self.slope * powered)
    }

    fn reserve(&self, supply: Uint128) -> Result<Uint128, CurveError> {
        // F(s) = slope * den / (num + den) * s^((num + den) / den)
        let s = self.normalize.from_supply(supply);
        let powered = pow_rational(s, self.integral_num(), self.exponent_den)?;
        let coefficient = self.slope * Decimal::from(self.exponent_den)
            / Decimal::from(self.integral_num());
        self.normalize.to_reserve(coefficient * powered)
    }

    fn supply(&self, reserve: Uint128) -> Result<Uint128, CurveError> {
        // F^-1(r) = ((num + den) * r / (slope * den))^(den / (num + den))
        if self.slope.is_zero() {
            return Err(CurveError::DivisionByZero);
        }
        let r = self.normalize.from_reserve(reserve);
        let numerator = Decimal::from(self.integral_num()) * r;
        let denominator = self.slope * Decimal::from(self.exponent_den);
        if denominator.is_zero() {
            return Err(CurveError::DivisionByZero);
        }
        let base = numerator / denominator;
        let supply = pow_rational(base, self.exponent_den, self.integral_num())?;
        self.normalize.to_supply(supply)
    }
}
