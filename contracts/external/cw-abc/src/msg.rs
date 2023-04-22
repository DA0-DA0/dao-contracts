use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal, Uint128, Decimal as StdDecimal};

use crate::abc::{CommonsPhaseConfig, CurveType, MinMax, ReserveToken, SupplyToken};


#[cw_serde]
pub struct InstantiateMsg {
    // Supply token information
    pub supply: SupplyToken,

    // Reserve token information
    pub reserve: ReserveToken,

    // Curve type for this contract
    pub curve_type: CurveType,

    // Hatch configuration information
    pub phase_config: CommonsPhaseConfig<String>,
}


#[cw_ownable::cw_ownable_execute]
#[cw_serde]
pub enum ExecuteMsg {
    /// Buy will attempt to purchase as many supply tokens as possible.
    /// You must send only reserve tokens in that message
    Buy {},
    /// Implements CW20. Burn is a base message to destroy tokens forever
    Burn { amount: Uint128 },
    /// Update the hatch phase allowlist
    UpdateHatchAllowlist {
        to_add: Vec<String>,
        to_remove: Vec<String>,
    },
    /// Update the hatch phase configuration
    /// This can only be called by the admin and only during the hatch phase
    UpdateHatchConfig {
        initial_raise: Option<MinMax>,
        initial_allocation_ratio: Option<StdDecimal>,
    },
}

#[cw_ownable::cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Returns the reserve and supply quantities, as well as the spot price to buy 1 token
    /// Returns [`CurveInfoResponse`]
    #[returns(CurveInfoResponse)]
    CurveInfo {},
    /// Returns the current phase configuration
    /// Returns [`CommonsPhaseConfigResponse`]
    #[returns(CommonsPhaseConfigResponse)]
    PhaseConfig {}
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
pub struct CommonsPhaseConfigResponse {
    // the phase configuration
    pub phase_config: CommonsPhaseConfig<Addr>,
}
