use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Binary, Uint128};
use cw_tokenfactory_issuer::msg::DenomUnit;
use cw_utils::Duration;
use dao_dao_macros::{active_query, token_query, voting_module_query};
use dao_voting::threshold::{ActiveThreshold, ActiveThresholdResponse};

#[cw_serde]
pub struct InitialBalance {
    pub amount: Uint128,
    pub address: String,
}

#[cw_serde]
pub struct NewDenomMetadata {
    /// The name of the token (e.g. "Cat Coin")
    pub name: String,
    /// The description of the token
    pub description: String,
    /// The ticker symbol of the token (e.g. "CAT")
    pub symbol: String,
    /// The unit commonly used in communication (e.g. "cat")
    pub display: String,
    /// Used define additional units of the token (e.g. "tiger")
    /// These must have an exponent larger than 0.
    pub additional_denom_units: Option<Vec<DenomUnit>>,
}

#[cw_serde]
pub struct NewTokenInfo {
    /// The code id of the cw-tokenfactory-issuer contract
    pub token_issuer_code_id: u64,
    /// The subdenom of the token to create, will also be used as an alias
    /// for the denom. The Token Factory denom will have the format of
    /// factory/{contract_address}/{subdenom}
    pub subdenom: String,
    /// Optional metadata for the token, this can additionally be set later.
    pub metadata: Option<NewDenomMetadata>,
    /// The initial balances to set for the token, cannot be empty.
    pub initial_balances: Vec<InitialBalance>,
    /// Optional balance to mint for the DAO.
    pub initial_dao_balance: Option<Uint128>,
}

#[cw_serde]
pub enum TokenInfo {
    /// Uses an existing Token Factory token and creates a new issuer contract.
    /// Full setup, such as transferring ownership or setting up MsgSetBeforeSendHook,
    /// must be done manually.
    Existing {
        /// Token factory denom
        denom: String,
    },
    /// Creates a new Token Factory token via the issue contract with the DAO automatically
    /// setup as admin and owner.
    New(NewTokenInfo),
    /// Uses a factory pattern that must return the denom, optionally a Token Contract address.
    /// The binary must serialize to a `WasmMsg::Execute` message.
    Factory(Binary),
}

#[cw_serde]
pub struct InstantiateMsg {
    /// New or existing native token to use for voting power.
    pub token_info: TokenInfo,
    /// How long until the tokens become liquid again
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
    UpdateConfig { duration: Option<Duration> },
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

#[active_query]
#[voting_module_query]
#[token_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(crate::state::Config)]
    GetConfig {},
    #[returns(DenomResponse)]
    Denom {},
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

#[cw_serde]
pub struct FactoryCallback {
    pub denom: String,
    pub token_contract: Option<String>,
}
