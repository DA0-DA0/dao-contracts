use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint128;
use cw_utils::Duration;
use dao_dao_macros::voting_module_query;
use dao_interface::state::Admin;

#[cw_serde]
pub struct InitialBalance {
    pub address: String,
    pub amount: Uint128,
}

#[cw_serde]
pub enum TokenInfo {
    Existing {
        /// Token denom e.g. ujuno, or some ibc denom.
        denom: String,
    },
    New {
        name: String,
        symbol: String,
        decimals: u32,
        initial_balances: Vec<InitialBalance>,

        initial_dao_balance: Option<Uint128>,
    },
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
}

#[cw_serde]
pub enum ExecuteMsg {
    Stake {},
    Unstake {
        amount: Uint128,
    },
    UpdateConfig {
        owner: Option<String>,
        manager: Option<String>,
        duration: Option<Duration>,
    },
    Claim {},
}

#[voting_module_query]
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
