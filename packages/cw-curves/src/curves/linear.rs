use cosmwasm_std::{Decimal as StdDecimal, Uint128};
use rust_decimal::Decimal;

use crate::{
    utils::{decimal_to_std, square_root},
    Curve, DecimalPlaces,
};

/// spot_price is slope * supply
pub struct Linear {
    pub slope: Decimal,
    pub normalize: DecimalPlaces,
}

impl Linear {
    pub fn new(slope: Decimal, normalize: DecimalPlaces) -> Self {
        Self { slope, normalize }
    }
}

impl Curve for Linear {
    fn spot_price(&self, supply: Uint128) -> StdDecimal {
        // f(x) = supply * self.value
        let out = self.normalize.from_supply(supply) * self.slope;
        decimal_to_std(out)
    }

    fn reserve(&self, supply: Uint128) -> Uint128 {
        // f(x) = self.slope * supply * supply / 2
        let normalized = self.normalize.from_supply(supply);
        let square = normalized * normalized;
        // Note: multiplying by 0.5 is much faster than dividing by 2
        let reserve = square * self.slope * Decimal::new(5, 1);
        self.normalize.to_reserve(reserve)
    }

    fn supply(&self, reserve: Uint128) -> Uint128 {
        // f(x) = (2 * reserve / self.slope) ^ 0.5
        // note: use addition here to optimize 2* operation
        let square = self.normalize.from_reserve(reserve + reserve) / self.slope;
        let supply = square_root(square);
        self.normalize.to_supply(supply)
    }
}
