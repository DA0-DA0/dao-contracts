use std::collections::HashSet;
use cosmwasm_schema::cw_serde;

use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::Item;
use crate::abc::{ CommonsPhaseConfig, CurveType, CommonsPhase};

use crate::curves::DecimalPlaces;

/// Supply is dynamic and tracks the current supply of staked and ERC20 tokens.
#[cw_serde]
pub struct CurveState {
    /// reserve is how many native tokens exist bonded to the validator
    pub reserve: Uint128,
    /// funding is how many native tokens exist unbonded and in the contract
    pub funding: Uint128,
    /// supply is how many tokens this contract has issued
    pub supply: Uint128,

    // the denom of the reserve token
    pub reserve_denom: String,

    // how to normalize reserve and supply
    pub decimals: DecimalPlaces,
}

impl CurveState {
    pub fn new(reserve_denom: String, decimals: DecimalPlaces) -> Self {
        CurveState {
            reserve: Uint128::zero(),
            funding: Uint128::zero(),
            supply: Uint128::zero(),
            reserve_denom,
            decimals,
        }
    }
}

pub const CURVE_STATE: Item<CurveState> = Item::new("curve_state");

pub const CURVE_TYPE: Item<CurveType> = Item::new("curve_type");

/// The denom used for the supply token
pub const SUPPLY_DENOM: Item<String> = Item::new("denom");

/// Keep track of who has contributed to the hatch phase
/// TODO: cw-set?
pub static HATCHERS: Item<HashSet<Addr>> = Item::new("hatchers");

/// The phase configuration of the Augmented Bonding Curve
pub static PHASE_CONFIG: Item<CommonsPhaseConfig<Addr>> = Item::new("phase_config");

/// The phase state of the Augmented Bonding Curve
pub static PHASE: Item<CommonsPhase> = Item::new("phase");

