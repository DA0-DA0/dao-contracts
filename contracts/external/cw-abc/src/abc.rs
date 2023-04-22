
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Api, Decimal as StdDecimal, ensure, StdResult, Uint128};
use cw_address_like::AddressLike;
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

/// Struct for minimium and maximum values
#[cw_serde]
pub struct MinMax {
    pub min: Uint128,
    pub max: Uint128,
}

#[cw_serde]
pub struct HatchConfig<T: AddressLike> {
    // Initial contributors (Hatchers) allow list
    pub allowlist: Option<Vec<T>>,
    // /// TODO: The minimum and maximum contribution amounts (min, max) in the reserve token
    // pub contribution_limits: MinMax,
    // The initial raise range (min, max) in the reserve token
    pub initial_raise: MinMax,
    // The initial price (p0) per reserve token
    // TODO: initial price is not implemented yet
    pub initial_price: Uint128,
    // The initial allocation (θ), percentage of the initial raise allocated to the Funding Pool
    pub initial_allocation_ratio: StdDecimal,
}

impl From<HatchConfig<Addr>> for HatchConfig<String> {
    fn from(value: HatchConfig<Addr>) -> Self {
        HatchConfig {
            allowlist: value.allowlist.map(|addresses| {
                addresses.into_iter().map(|addr| addr.to_string()).collect()
            }),
            initial_raise: value.initial_raise,
            initial_price: value.initial_price,
            initial_allocation_ratio: value.initial_allocation_ratio,
        }
    }
}


impl HatchConfig<String> {
    /// Validate the hatch config
    pub fn validate(&self, api: &dyn Api) -> Result<HatchConfig<Addr>, ContractError> {
        ensure!(
            self.initial_raise.min < self.initial_raise.max,
            ContractError::HatchPhaseConfigError("Initial raise minimum value must be less than maximum value.".to_string())
        );

        ensure!(
            !self.initial_price.is_zero(),
            ContractError::HatchPhaseConfigError("Initial price must be greater than zero.".to_string())
        );

        ensure!(
            self.initial_allocation_ratio <= StdDecimal::percent(100u64),
            ContractError::HatchPhaseConfigError("Initial allocation percentage must be between 0 and 100.".to_string())
        );

        let allowlist = self
            .allowlist
            .as_ref()
            .map(|addresses| {
                addresses
                    .iter()
                    .map(|addr| api.addr_validate(addr))
                    .collect::<StdResult<Vec<_>>>()
            })
            .transpose()?;

        Ok(HatchConfig {
            allowlist,
            initial_raise: self.initial_raise.clone(),
            initial_price: self.initial_price,
            initial_allocation_ratio: self.initial_allocation_ratio,
        })
    }
}

impl HatchConfig<Addr> {
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
pub struct CommonsPhaseConfig<T: AddressLike> {
    // The Hatch phase where initial contributors (Hatchers) participate in a hatch sale.
    pub hatch: HatchConfig<T>,
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

impl CommonsPhaseConfig<String> {
    /// Validate that the commons configuration is valid
    pub fn validate(&self, api: &dyn Api) -> Result<CommonsPhaseConfig<Addr>, ContractError> {
        let hatch = self.hatch.validate(api)?;
        self.open.validate()?;

        Ok(CommonsPhaseConfig {
            hatch,
            open: self.open.clone(),
            closed: self.closed.clone(),
        })
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

