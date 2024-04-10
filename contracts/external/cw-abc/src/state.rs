use cosmwasm_schema::cw_serde;
use dao_interface::token::NewTokenInfo;

use crate::abc::{CommonsPhase, CommonsPhaseConfig, CurveType};
use cosmwasm_std::{Addr, Empty, Uint128};
use cw_curves::DecimalPlaces;
use cw_storage_plus::{Item, Map};

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

/// The paused state for implementing a circuit breaker
pub const IS_PAUSED: Item<bool> = Item::new("is_paused");

pub const CURVE_STATE: Item<CurveState> = Item::new("curve_state");

pub const CURVE_TYPE: Item<CurveType> = Item::new("curve_type");

/// The address for automatically forwarding funding pool gains
pub const FUNDING_POOL_FORWARDING: Item<Addr> = Item::new("funding_pool_forwarding");

/// The denom used for the supply token
pub const SUPPLY_DENOM: Item<String> = Item::new("denom");

/// The initial supply of the supply token when the ABC was created
pub const INITIAL_SUPPLY: Item<Uint128> = Item::new("initial_supply");

/// The maximum supply of the supply token, new tokens cannot be minted beyond this cap
pub const MAX_SUPPLY: Item<Uint128> = Item::new("max_supply");

/// Hatcher phase allowlist
pub static HATCHER_ALLOWLIST: Map<&Addr, Empty> = Map::new("hatcher_allowlist");

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

/// Temporarily holds NewTokenInfo when creating a new Token Factory denom
pub const NEW_TOKEN_INFO: Item<NewTokenInfo> = Item::new("new_token_info");

/// The address of the cw-tokenfactory-issuer contract
pub const TOKEN_ISSUER_CONTRACT: Item<Addr> = Item::new("token_issuer_contract");
