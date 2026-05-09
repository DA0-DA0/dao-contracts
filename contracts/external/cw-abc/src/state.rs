use std::fmt::{self, Display};

use crate::abc::{CommonsPhase, CommonsPhaseConfig, CurveType, MinMax, SupplyToken};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128, Uint64};
use cw_curves::DecimalPlaces;
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, Map, MultiIndex};

/// Supply is dynamic and tracks the current supply of staked and ERC20 tokens.
#[cw_serde]
pub struct CurveState {
    /// reserve is how many native tokens exist bonded to the validator
    pub reserve: Uint128,
    /// funding is how many native tokens exist unbonded and in the contract
    pub funding: Uint128,
    /// supply is how many tokens this contract has issued
    pub supply: Uint128,

    /// the denom of the reserve token
    pub reserve_denom: String,

    /// how to normalize reserve and supply
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

/// The configuration for a member of the hatcher allowlist
#[cw_serde]
pub struct HatcherAllowlistConfig {
    /// The type of the configuration
    pub config_type: HatcherAllowlistConfigType,
    /// An optional override of the hatch_config's contribution limit
    pub contribution_limits_override: Option<MinMax>,
    /// The height of the config insertion
    /// For use when checking allowlist of DAO configs
    pub config_height: u64,
}

#[cw_serde]
pub struct HatcherAllowlistEntry {
    pub addr: Addr,
    pub config: HatcherAllowlistConfig,
}

#[cw_serde]
pub enum HatcherAllowlistConfigType {
    DAO {
        /// The optional priority for checking a DAO config
        /// None will append the item to the end of the priority queue (least priority)
        priority: Option<Uint64>,
    },
    Address {},
}

impl Copy for HatcherAllowlistConfigType {}

impl Display for HatcherAllowlistConfigType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HatcherAllowlistConfigType::DAO { priority: _ } => write!(f, "DAO"),
            HatcherAllowlistConfigType::Address {} => write!(f, "Address"),
        }
    }
}

pub struct HatcherAllowlistIndexes<'a> {
    pub config_type: MultiIndex<'a, String, HatcherAllowlistConfig, &'a Addr>,
}

impl<'a> IndexList<HatcherAllowlistConfig> for HatcherAllowlistIndexes<'a> {
    fn get_indexes(
        &'_ self,
    ) -> Box<dyn Iterator<Item = &'_ dyn cw_storage_plus::Index<HatcherAllowlistConfig>> + '_> {
        let v: Vec<&dyn Index<HatcherAllowlistConfig>> = vec![&self.config_type];
        Box::new(v.into_iter())
    }
}

pub fn hatcher_allowlist<'a>(
) -> IndexedMap<'a, &'a Addr, HatcherAllowlistConfig, HatcherAllowlistIndexes<'a>> {
    let indexes = HatcherAllowlistIndexes {
        config_type: MultiIndex::new(
            |_, x: &HatcherAllowlistConfig| x.config_type.to_string(),
            "hatcher_allowlist",
            "hatcher_allowlist__config_type",
        ),
    };

    IndexedMap::new("hatcher_allowlist", indexes)
}

/// The hatcher allowlist with configurations
pub const HATCHER_ALLOWLIST: Map<&Addr, HatcherAllowlistConfig> = Map::new("hatcher_allowlist");

/// The DAO portion of the hatcher allowlist implemented as a priority queue
/// If someone is a member of multiple allowlisted DAO's, we want to be able to control the checking order
pub const HATCHER_DAO_PRIORITY_QUEUE: Item<Vec<HatcherAllowlistEntry>> =
    Item::new("HATCHER_DAO_PRIORITY_QUEUE");

/// The paused state for implementing a circuit breaker
pub const IS_PAUSED: Item<bool> = Item::new("is_paused");

pub const CURVE_STATE: Item<CurveState> = Item::new("curve_state");

pub const CURVE_TYPE: Item<CurveType> = Item::new("curve_type");

/// The address for automatically forwarding funding pool gains
pub const FUNDING_POOL_FORWARDING: Item<Addr> = Item::new("funding_pool_forwarding");

/// The denom used for the supply token
pub const SUPPLY_DENOM: Item<String> = Item::new("denom");

/// The maximum supply of the supply token, new tokens cannot be minted beyond this cap
pub const MAX_SUPPLY: Item<Uint128> = Item::new("max_supply");

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

/// Temporarily holds the supply config when creating a new Token Factory denom
pub const TEMP_SUPPLY: Item<SupplyToken> = Item::new("temp_supply");

/// The address of the cw-tokenfactory-issuer contract
pub const TOKEN_ISSUER_CONTRACT: Item<Addr> = Item::new("token_issuer_contract");
