use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint128;
use cw_utils::Duration;
use dao_dao_macros::{active_query, voting_module_query};
use dao_interface::state::Admin;
use dao_voting::threshold::{ActiveThreshold, ActiveThresholdResponse};
use token_bindings::Metadata;

#[cw_serde]
pub struct InitialBalance {
    pub amount: Uint128,
    pub address: String,
}

#[cw_serde]
pub struct NewTokenInfo {
    // TODO
    // pub token_issuer_code_id: u64,
    pub subdenom: String,
    pub metadata: Option<Metadata>,
    pub initial_balances: Vec<InitialBalance>,
    pub initial_dao_balance: Option<Uint128>,
}

#[cw_serde]
pub enum TokenInfo {
    Existing {
        /// Token denom e.g. ujuno, or some ibc denom.
        denom: String,
    },
    New(NewTokenInfo),
}

#[cw_serde]
pub struct InstantiateMsg {
    // Owner can update all configs including changing the owner. This will generally be a DAO.
    pub owner: Option<Admin>,
    // Manager can update all configs except changing the owner. This will generally be an operations multisig for a DAO.
    pub manager: Option<String>,
    // New or existing native token to use for voting power.
    pub token_info: TokenInfo,
    // How long until the tokens become liquid again
    pub unstaking_duration: Option<Duration>,
    /// The number or percentage of tokens that must be staked
    /// for the DAO to be active
    pub active_threshold: Option<ActiveThreshold>,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Stakes tokens with the contract to get voting power in the DAO
    Stake {},
    /// Unstakes tokens so that they begin unbonding
    Unstake { amount: Uint128 },
    /// Updates the contract configuration
    UpdateConfig {
        owner: Option<String>,
        manager: Option<String>,
        duration: Option<Duration>,
    },
    /// Claims unstaked tokens that have completed the unbonding period
    Claim {},
    /// Sets the active threshold to a new value. Only the
    /// instantiator of this contract (a DAO most likely) may call this
    /// method.
    UpdateActiveThreshold {
        new_threshold: Option<ActiveThreshold>,
    },
    /// Adds a hook that fires on staking / unstaking
    AddHook { addr: String },
    /// Removes a hook that fires on staking / unstaking
    RemoveHook { addr: String },
}

#[voting_module_query]
#[active_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(crate::state::Config)]
    GetConfig {},
    #[returns(DenomResponse)]
    GetDenom {},
    #[returns(cw_controllers::ClaimsResponse)]
    Claims { address: String },
    #[returns(ListStakersResponse)]
    ListStakers {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    #[returns(ActiveThresholdResponse)]
    ActiveThreshold {},
    #[returns(GetHooksResponse)]
    GetHooks {},
}

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
pub struct ListStakersResponse {
    pub stakers: Vec<StakerBalanceResponse>,
}

#[cw_serde]
pub struct StakerBalanceResponse {
    pub address: String,
    pub balance: Uint128,
}

#[cw_serde]
pub struct DenomResponse {
    pub denom: String,
}

#[cw_serde]
pub struct GetHooksResponse {
    pub hooks: Vec<String>,
}
