use cosmwasm_std::{Decimal as StdDecimal, Uint128};
use rust_decimal::Decimal;

use crate::{utils::decimal_to_std, Curve, CurveError, DecimalPlaces};

/// spot price is always a constant value
pub struct Constant {
    pub value: Decimal,
    pub normalize: DecimalPlaces,
}

impl Constant {
    pub fn new(value: Decimal, normalize: DecimalPlaces) -> Self {
        Self { value, normalize }
    }
}

impl Curve for Constant {
    // we need to normalize value with the reserve decimal places
    // (eg 0.1 value would return 100_000 if reserve was uatom)
    fn spot_price(&self, _supply: Uint128) -> Result<StdDecimal, CurveError> {
        // f(x) = self.value
        decimal_to_std(self.value)
    }

    /// Returns total number of reserve tokens needed to purchase a given number of supply tokens.
    /// Note that both need to be normalized.
    fn reserve(&self, supply: Uint128) -> Result<Uint128, CurveError> {
        // f(x) = supply * self.value
        let reserve = self.normalize.from_supply(supply) * self.value;
        self.normalize.to_reserve(reserve)
    }

    fn supply(&self, reserve: Uint128) -> Result<Uint128, CurveError> {
        // f(x) = reserve / self.value
        if self.value.is_zero() {
            return Err(CurveError::DivisionByZero);
        }
        let supply = self.normalize.from_reserve(reserve) / self.value;
        self.normalize.to_supply(supply)
    }
}
