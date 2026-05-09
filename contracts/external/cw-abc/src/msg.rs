use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal, Uint128};

use crate::{
    abc::{CommonsPhase, CommonsPhaseConfig, CurveType, MinMax, ReserveToken, SupplyToken},
    state::{HatcherAllowlistConfigType, HatcherAllowlistEntry},
};

#[cw_serde]
pub struct InstantiateMsg {
    /// The code id of the cw-tokenfactory-issuer contract
    pub token_issuer_code_id: u64,

    /// An optional address for automatically forwarding funding pool gains
    pub funding_pool_forwarding: Option<String>,

    /// Supply token information
    pub supply: SupplyToken,

    /// Reserve token information
    pub reserve: ReserveToken,

    /// Curve type for this contract
    pub curve_type: CurveType,

    /// Hatch configuration information
    pub phase_config: CommonsPhaseConfig,

    /// TODO different ways of doing this, for example DAO members?
    /// Using a whitelist contract? Merkle tree?
    /// Hatcher allowlist
    pub hatcher_allowlist: Option<Vec<HatcherAllowlistEntryMsg>>,
}

/// Update the phase configurations.
/// These can only be called by the owner.
#[cw_serde]
pub enum UpdatePhaseConfigMsg {
    /// Update the hatch phase configuration
    Hatch {
        contribution_limits: Option<MinMax>,
        // TODO what is the minimum used for?
        initial_raise: Option<MinMax>,
        entry_fee: Option<Decimal>,
    },
    /// Update the open phase configuration.
    Open {
        exit_fee: Option<Decimal>,
        entry_fee: Option<Decimal>,
    },
    /// Update the closed phase configuration.
    /// TODO Set the curve type to be used on close?
    Closed {},
}

#[cw_ownable::cw_ownable_execute]
#[cw_serde]
pub enum ExecuteMsg {
    /// Buy will attempt to purchase as many supply tokens as possible.
    /// You must send only reserve tokens.
    Buy {},
    /// Sell burns supply tokens in return for the reserve token.
    /// You must send only supply tokens.
    Sell {},
    /// Donate will donate tokens to the funding pool.
    /// You must send only reserve tokens.
    Donate {},
    /// Withdraw will withdraw tokens from the funding pool.
    Withdraw {
        /// The amount to withdraw (defaults to full amount).
        amount: Option<Uint128>,
    },
    /// Sets (or unsets if set to None) the maximum supply
    UpdateMaxSupply {
        /// The maximum supply able to be minted.
        max_supply: Option<Uint128>,
    },
    /// Updates the curve type used for pricing tokens.
    /// Only callable by owner.
    /// TODO think about other potential limitations on this.
    UpdateCurve { curve_type: CurveType },
    /// Update the hatch phase allowlist.
    /// Only callable by owner.
    UpdateHatchAllowlist {
        /// Addresses to be added.
        to_add: Vec<HatcherAllowlistEntryMsg>,
        /// Addresses to be removed.
        to_remove: Vec<String>,
    },
    /// Toggles the paused state (circuit breaker)
    TogglePause {},
    /// Update the funding pool forwarding.
    /// Only callable by owner.
    UpdateFundingPoolForwarding {
        /// The address to receive the funding pool forwarding.
        /// Set to None to stop forwarding.
        address: Option<String>,
    },
    /// Update the configuration of a certain phase.
    /// This can only be called by the owner.
    UpdatePhaseConfig(UpdatePhaseConfigMsg),
    /// Closing the bonding curve means no more buys are enabled and exit tax is set
    /// to zero.
    /// For example, this could be used in the event of a project shutting down.
    Close {},
}

#[cw_ownable::cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Returns the reserve and supply quantities, as well as the spot price to buy 1 token
    /// Returns [`CurveInfoResponse`]
    #[returns(CurveInfoResponse)]
    CurveInfo {},
    /// Returns information about the curve type (i.e. linear, constant, etc.)
    #[returns(CurveType)]
    CurveType {},
    /// Returns Token Factory Denom for the supply
    #[returns(DenomResponse)]
    Denom {},
    /// Returns a list of the donors and their donations
    /// Returns [`DonationsResponse`]
    #[returns(DonationsResponse)]
    Donations {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    #[returns(bool)]
    IsPaused {},
    /// Returns the funding pool forwarding config for the contract. This is the address that
    /// receives any fees collected from bonding curve operation and donations
    #[returns(Option<::cosmwasm_std::Addr>)]
    FundingPoolForwarding {},
    /// List the hatchers and their contributions
    /// Returns [`HatchersResponse`]
    #[returns(HatchersResponse)]
    Hatchers {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// Returns the contribution of a hatcher
    #[returns(Uint128)]
    Hatcher { addr: String },
    /// Lists the hatcher allowlist
    /// Returns [`HatcherAllowlistResponse`]
    #[returns(HatcherAllowlistResponse)]
    HatcherAllowlist {
        start_after: Option<String>,
        limit: Option<u32>,
        config_type: Option<HatcherAllowlistConfigType>,
    },
    /// Returns the Maximum Supply of the supply token
    #[returns(Uint128)]
    MaxSupply {},
    /// Returns the amount of tokens to receive from buying
    #[returns(QuoteResponse)]
    BuyQuote { payment: Uint128 },
    /// Returns the amount of tokens to receive from selling
    #[returns(QuoteResponse)]
    SellQuote { payment: Uint128 },
    /// Returns the current phase
    #[returns(CommonsPhase)]
    Phase {},
    /// Returns the current phase configuration
    /// Returns [`CommonsPhaseConfigResponse`]
    #[returns(CommonsPhaseConfigResponse)]
    PhaseConfig {},
    /// Returns the address of the cw-tokenfactory-issuer contract
    #[returns(::cosmwasm_std::Addr)]
    TokenContract {},
}

#[cw_serde]
pub struct HatcherAllowlistEntryMsg {
    pub addr: String,
    pub config: HatcherAllowlistConfigMsg,
}

#[cw_serde]
pub struct HatcherAllowlistConfigMsg {
    /// The type of the configuration
    pub config_type: HatcherAllowlistConfigType,
    /// An optional override of the hatch_config's contribution limit
    pub contribution_limits_override: Option<MinMax>,
}

#[cw_serde]
pub struct CurveInfoResponse {
    /// How many reserve tokens have been received
    pub reserve: Uint128,
    /// How many supply tokens have been issued
    pub supply: Uint128,
    /// The amount of tokens in the funding pool
    pub funding: Uint128,
    /// Current spot price of the token
    pub spot_price: Decimal,
    /// Current reserve denom
    pub reserve_denom: String,
}

#[cw_serde]
pub struct DenomResponse {
    pub denom: String,
}

#[cw_serde]
pub struct HatcherAllowlistResponse {
    /// Hatcher allowlist
    pub allowlist: Option<Vec<HatcherAllowlistEntry>>,
}

#[cw_serde]
pub struct CommonsPhaseConfigResponse {
    /// The phase configuration
    pub phase_config: CommonsPhaseConfig,

    /// Current phase
    pub phase: CommonsPhase,
}

#[cw_serde]
pub struct DonationsResponse {
    /// The donators mapped to their donation in the reserve token
    pub donations: Vec<(Addr, Uint128)>,
}

#[cw_serde]
pub struct HatchersResponse {
    /// The hatchers mapped to their contribution in the reserve token
    pub hatchers: Vec<(Addr, Uint128)>,
}

#[cw_serde]
pub struct QuoteResponse {
    pub new_reserve: Uint128,
    pub funded: Uint128,
    pub amount: Uint128,
    pub new_supply: Uint128,
}

#[cw_serde]
pub struct MigrateMsg {}
