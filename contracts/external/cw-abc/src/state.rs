use cosmwasm_schema::cw_serde;
use std::collections::HashSet;

use crate::abc::{CommonsPhase, CommonsPhaseConfig, CurveType};
use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map};

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

/// Hatcher phase allowlist
/// TODO: we could use the keys for the [`HATCHERS`] map instead setting them to 0 at the beginning, though existing hatchers would not be able to be removed
pub static HATCHER_ALLOWLIST: Item<HashSet<Addr>> = Item::new("hatch_allowlist");

/// Keep track of who has contributed to the hatch phase
/// TODO: cw-set? This should be a map because in the open-phase we need to be able
/// to ascertain the amount contributed by a user
pub static HATCHERS: Map<&Addr, Uint128> = Map::new("hatchers");

/// Keep track of the donated amounts per user
pub static DONATIONS: Map<&Addr, Uint128> = Map::new("donations");

/// The phase configuration of the Augmented Bonding Curve
pub static PHASE_CONFIG: Item<CommonsPhaseConfig> = Item::new("phase_config");

/// The phase state of the Augmented Bonding Curve
pub static PHASE: Item<CommonsPhase> = Item::new("phase");
