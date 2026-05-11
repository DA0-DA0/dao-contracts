use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Decimal as StdDecimal, Uint128};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use thiserror::Error;

use crate::utils::decimal;

/// Errors that can occur during curve evaluation. L-5: replaces the
/// previous `unwrap()` panics with typed errors so consumers can choose
/// to surface them rather than crash the contract.
#[derive(Error, Debug, PartialEq)]
pub enum CurveError {
    #[error("curve math overflow at scale {scale}, value {value}")]
    Overflow { scale: u32, value: String },
    #[error("curve division by zero")]
    DivisionByZero,
}

/// This defines the curves we are using.
///
/// I am struggling on what type to use for the math. Tokens are often stored as Uint128,
/// but they may have 6 or 9 digits. When using constant or linear functions, this doesn't matter
/// much, but for non-linear functions a lot more. Also, supply and reserve most likely have different
/// decimals... either we leave it for the callers to normalize and accept a `Decimal` input,
/// or we pass in `Uint128` as well as the decimal places for supply and reserve.
///
/// After working the first route and realizing that `Decimal` is not all that great to work with
/// when you want to do more complex math than add and multiply `Uint128`, I decided to go the second
/// route. That made the signatures quite complex and my final idea was to pass in `supply_decimal`
/// and `reserve_decimal` in the curve constructors.
pub trait Curve {
    /// Returns the spot price given the supply.
    /// `f(x)` from the README.
    fn spot_price(&self, supply: Uint128) -> Result<StdDecimal, CurveError>;

    /// Returns the total price paid up to purchase supply tokens (integral)
    /// `F(x)` from the README.
    fn reserve(&self, supply: Uint128) -> Result<Uint128, CurveError>;

    /// Inverse of reserve. Returns how many tokens would be issued
    /// with a total paid amount of reserve.
    /// `F^-1(x)` from the README.
    fn supply(&self, reserve: Uint128) -> Result<Uint128, CurveError>;
}

/// DecimalPlaces should be passed into curve constructors
#[cw_serde]
#[derive(Copy)]
pub struct DecimalPlaces {
    /// Number of decimal places for the supply token (this is what was passed in cw20-base instantiate
    pub supply: u32,
    /// Number of decimal places for the reserve token (eg. 6 for uatom, 9 for nstep, 18 for wei)
    pub reserve: u32,
}

impl DecimalPlaces {
    pub fn new(supply: u8, reserve: u8) -> Self {
        DecimalPlaces {
            supply: supply as u32,
            reserve: reserve as u32,
        }
    }

    pub fn to_reserve(self, reserve: Decimal) -> Result<Uint128, CurveError> {
        let factor = decimal(10u128.pow(self.reserve), 0);
        let out = reserve * factor;
        out.floor()
            .to_u128()
            .map(Uint128::from)
            .ok_or(CurveError::Overflow {
                scale: self.reserve,
                value: reserve.to_string(),
            })
    }

    pub fn to_supply(self, supply: Decimal) -> Result<Uint128, CurveError> {
        let factor = decimal(10u128.pow(self.supply), 0);
        let out = supply * factor;
        out.floor()
            .to_u128()
            .map(Uint128::from)
            .ok_or(CurveError::Overflow {
                scale: self.supply,
                value: supply.to_string(),
            })
    }

    pub fn from_supply(&self, supply: Uint128) -> Decimal {
        decimal(supply, self.supply)
    }

    pub fn from_reserve(&self, reserve: Uint128) -> Decimal {
        decimal(reserve, self.reserve)
    }
}
