use cosmwasm_schema::cw_serde;
use cosmwasm_std::{ensure, Uint128};
use token_bindings::Metadata;
use crate::curves::{Constant, Curve, decimal, DecimalPlaces, Linear, SquareRoot};
use crate::ContractError;

#[cw_serde]
pub struct SupplyToken {
    // The denom to create for the supply token
    pub subdenom: String,
    // Metadata for the supply token to create
    pub metadata: Metadata,
    // Number of decimal places for the reserve token, needed for proper curve math.
    // Same format as decimals above, eg. if it is uatom, where 1 unit is 10^-6 ATOM, use 6 here
    pub decimals: u8,
}

#[cw_serde]
pub struct ReserveToken {
    // Reserve token denom (only support native for now)
    pub denom: String,
    // Number of decimal places for the reserve token, needed for proper curve math.
    // Same format as decimals above, eg. if it is uatom, where 1 unit is 10^-6 ATOM, use 6 here
    pub decimals: u8,
}


#[cw_serde]
pub enum CommonsPhase {
    // The Hatch phase where initial contributors (Hatchers) participate in a hatch sale.
    Hatch(HatchConfig),
    // The Vesting phase where tokens minted during the Hatch phase are locked (burning is disabled) to combat early speculation/arbitrage.
    Vesting {
        // The start timestamp of the vesting process.
        vesting_start: u64,
        // The end timestamp of the vesting process.
        vesting_end: u64,
    },
    // The Open phase where anyone can mint tokens by contributing DAI into the curve and becoming members of the Commons.
    Open,
}


#[cw_serde]
pub struct HatchConfig {
    // Initial contributors (Hatchers) list
    // pub hatchers: Vec<Addr>,
    // The initial raise range (min, max) in the reserve token
    pub initial_raise: (Uint128, Uint128),
    // The initial price (p0) per reserve token
    pub initial_price: Uint128,
    // The initial allocation (θ), percentage of the initial raise allocated to the Funding Pool
    pub initial_allocation: u8,
    // Percentage of capital put into the Reserve Pool during the Hatch phase
    pub reserve_percentage: u8,
}

impl HatchConfig {
    pub fn validate(&self) -> Result<(), ContractError> {
        // ensure!(!self.hatchers.is_empty(), ContractError::HatchConfigError("Hatchers list must not be empty."));

        ensure!(
            self.initial_raise.0 < self.initial_raise.1,
            ContractError::HatchConfigError("Initial raise minimum value must be less than maximum value.".to_string())
        );

        ensure!(
            !self.initial_price.is_zero(),
            ContractError::HatchConfigError("Initial price must be greater than zero.".to_string())
        );

        ensure!(
            self.initial_allocation <= 100,
            ContractError::HatchConfigError("Initial allocation percentage must be between 0 and 100.".to_string())
        );

        ensure!(
            self.reserve_percentage <= 100,
            ContractError::HatchConfigError("Reserve percentage must be between 0 and 100.".to_string())
        );

        Ok(())
    }
}


pub type CurveFn = Box<dyn Fn(DecimalPlaces) -> Box<dyn Curve>>;

#[cw_serde]
pub enum CurveType {
    /// Constant always returns `value * 10^-scale` as spot price
    Constant { value: Uint128, scale: u32 },
    /// Linear returns `slope * 10^-scale * supply` as spot price
    Linear { slope: Uint128, scale: u32 },
    /// SquareRoot returns `slope * 10^-scale * supply^0.5` as spot price
    SquareRoot { slope: Uint128, scale: u32 },
}

impl CurveType {
    pub fn to_curve_fn(&self) -> CurveFn {
        match self.clone() {
            CurveType::Constant { value, scale } => {
                let calc = move |places| -> Box<dyn Curve> {
                    Box::new(Constant::new(decimal(value, scale), places))
                };
                Box::new(calc)
            }
            CurveType::Linear { slope, scale } => {
                let calc = move |places| -> Box<dyn Curve> {
                    Box::new(Linear::new(decimal(slope, scale), places))
                };
                Box::new(calc)
            }
            CurveType::SquareRoot { slope, scale } => {
                let calc = move |places| -> Box<dyn Curve> {
                    Box::new(SquareRoot::new(decimal(slope, scale), places))
                };
                Box::new(calc)
            }
        }
    }
}

