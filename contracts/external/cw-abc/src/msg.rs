use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal, Decimal as StdDecimal, Uint128};

use crate::abc::{CommonsPhase, CommonsPhaseConfig, CurveType, MinMax, ReserveToken, SupplyToken};

#[cw_serde]
pub struct InstantiateMsg {
    // Supply token information
    pub supply: SupplyToken,

    // Reserve token information
    pub reserve: ReserveToken,

    // Curve type for this contract
    pub curve_type: CurveType,

    // Hatch configuration information
    pub phase_config: CommonsPhaseConfig,

    // Hatcher allowlist
    pub hatcher_allowlist: Option<Vec<String>>,
}

/// Update the phase configurations.
/// These can only be called by the admin and only before or during each phase
#[cw_serde]
pub enum UpdatePhaseConfigMsg {
    /// Update the hatch phase configuration
    Hatch {
        initial_raise: Option<MinMax>,
        initial_allocation_ratio: Option<StdDecimal>,
    },
    /// Update the open phase configuration
    Open {
        exit_tax: Option<StdDecimal>,
        reserve_ratio: Option<StdDecimal>,
    },
    /// Update the closed phase configuration
    Closed {},
}

#[cw_ownable::cw_ownable_execute]
#[cw_serde]
#[cfg_attr(feature = "boot", derive(boot_core::ExecuteFns))]
pub enum ExecuteMsg {
    /// Buy will attempt to purchase as many supply tokens as possible.
    /// You must send only reserve tokens in that message
    #[payable]
    Buy {},
    /// Burn is a base message to destroy tokens forever
    #[payable]
    Burn {},
    /// Donate will add reserve tokens to the funding pool
    #[payable]
    Donate {},
    /// Update the hatch phase allowlist
    UpdateHatchAllowlist {
        to_add: Vec<String>,
        to_remove: Vec<String>,
    },
    /// Update the hatch phase configuration
    /// This can only be called by the admin and only during the hatch phase
    UpdatePhaseConfig(UpdatePhaseConfigMsg),
}

#[cw_ownable::cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses)]
#[cfg_attr(feature = "boot", derive(boot_core::QueryFns))]
pub enum QueryMsg {
    /// Returns the reserve and supply quantities, as well as the spot price to buy 1 token
    /// Returns [`CurveInfoResponse`]
    #[returns(CurveInfoResponse)]
    CurveInfo {},
    /// Returns the current phase configuration
    /// Returns [`CommonsPhaseConfigResponse`]
    #[returns(CommonsPhaseConfigResponse)]
    PhaseConfig {},
    /// Returns a list of the donors and their donations
    /// Returns [`DonationsResponse`]
    #[returns(DonationsResponse)]
    Donations {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// List the hatchers and their contributions
    /// Returns [`HatchersResponse`]
    #[returns(HatchersResponse)]
    Hatchers {
        start_after: Option<String>,
        limit: Option<u32>,
    },
}

#[cw_serde]
pub struct CurveInfoResponse {
    // how many reserve tokens have been received
    pub reserve: Uint128,
    // how many supply tokens have been issued
    pub supply: Uint128,
    // the amount of tokens in the funding pool
    pub funding: Uint128,
    // current spot price of the token
    pub spot_price: Decimal,
    // current reserve denom
    pub reserve_denom: String,
}

#[cw_serde]
pub struct HatcherAllowlistResponse {
    // hatcher allowlist
    pub allowlist: Option<Vec<Addr>>,
}

#[cw_serde]
pub struct CommonsPhaseConfigResponse {
    // the phase configuration
    pub phase_config: CommonsPhaseConfig,

    // current phase
    pub phase: CommonsPhase,
}

#[cw_serde]
pub struct DonationsResponse {
    // the donators mapped to their donation in the reserve token
    pub donations: Vec<(Addr, Uint128)>,
}

#[cw_serde]
pub struct HatchersResponse {
    // the hatchers mapped to their contribution in the reserve token
    pub hatchers: Vec<(Addr, Uint128)>,
}
