
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal as StdDecimal, ensure, Uint128};
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
pub struct HatchConfig {
    // Initial contributors (Hatchers) allow list
    pub allowlist: Option<Vec<Addr>>,
    // The initial raise range (min, max) in the reserve token
    pub initial_raise: (Uint128, Uint128),
    // The initial price (p0) per reserve token
    pub initial_price: Uint128,
    // The initial allocation (θ), percentage of the initial raise allocated to the Funding Pool
    pub initial_allocation_percentage: StdDecimal,
}

impl HatchConfig {
    /// Validate the hatch config
    pub fn validate(&self) -> Result<(), ContractError> {
        ensure!(
            self.initial_raise.0 < self.initial_raise.1,
            ContractError::HatchPhaseConfigError("Initial raise minimum value must be less than maximum value.".to_string())
        );

        ensure!(
            !self.initial_price.is_zero(),
            ContractError::HatchPhaseConfigError("Initial price must be greater than zero.".to_string())
        );

        ensure!(
            self.initial_allocation_percentage <= StdDecimal::percent(100u64),
            ContractError::HatchPhaseConfigError("Initial allocation percentage must be between 0 and 100.".to_string())
        );

        Ok(())
    }

    /// Check if the sender is allowlisted for the hatch phase
    pub fn assert_allowlisted(&self, hatcher: &Addr) -> Result<(), ContractError> {
        if let Some(allowlist) = &self.allowlist {
            ensure!(
                allowlist.contains(hatcher),
                ContractError::SenderNotAllowlisted {
                    sender: hatcher.to_string(),
                }
            );
        }

        Ok(())
    }
}


#[cw_serde]
pub struct OpenConfig {
    // Percentage of capital put into the Reserve Pool during the Open phase
    pub allocation_percentage: StdDecimal,
}

impl OpenConfig {
    /// Validate the open config
    pub fn validate(&self) -> Result<(), ContractError> {

        ensure!(
            self.allocation_percentage <= StdDecimal::percent(100u64),
            ContractError::OpenPhaseConfigError("Reserve percentage must be between 0 and 100.".to_string())
        );

        Ok(())
    }
}

#[cw_serde]
pub struct ClosedConfig {}


#[cw_serde]
pub struct CommonsPhaseConfig {
    // The Hatch phase where initial contributors (Hatchers) participate in a hatch sale.
    pub hatch: HatchConfig,
    // The Vesting phase where tokens minted during the Hatch phase are locked (burning is disabled) to combat early speculation/arbitrage.
    // pub vesting: VestingConfig,
    // The Open phase where anyone can mint tokens by contributing the reserve token into the curve and becoming members of the Commons.
    pub open: OpenConfig,
    // The Closed phase where the Commons is closed to new members.
    pub closed: ClosedConfig,
}

// #[derive(Default)]
// #[cw_serde]
// pub struct HatchPhaseState {
//     // Initial contributors (Hatchers)
//     pub hatchers: HashSet<Addr>,
// }
//
// // TODO: maybe should be combined with config or just placed in state
// #[cw_serde]
// pub struct CommonsPhaseState {
//     pub hatch: HatchPhaseState,
//     // Vesting,
//     pub open: (),
//     // TODO: should we allow for a closed phase?
//     pub closed: ()
// }

#[cw_serde]
pub enum CommonsPhase {
    Hatch,
    Open,
    // TODO: should we allow for a closed phase?
    Closed
}

impl CommonsPhaseConfig {
    pub fn validate(&self) -> Result<(), ContractError> {
        self.hatch.validate()?;

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

