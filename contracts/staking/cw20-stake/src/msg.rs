use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint128;
use cw20::Cw20ReceiveMsg;
pub use cw_controllers::ClaimsResponse;
use cw_utils::Duration;

#[cw_serde]
pub struct InstantiateMsg {
    // Owner can update all configs including changing the owner. This will generally be a DAO.
    pub owner: Option<String>,
    // Manager can update all configs except changing the owner. This will generally be an operations multisig for a DAO.
    pub manager: Option<String>,
    pub token_address: String,
    pub unstaking_duration: Option<Duration>,
}

#[cw_serde]
pub enum ExecuteMsg {
    Receive(Cw20ReceiveMsg),
    Unstake {
        amount: Uint128,
    },
    Claim {},
    UpdateConfig {
        owner: Option<String>,
        manager: Option<String>,
        duration: Option<Duration>,
    },
    AddHook {
        addr: String,
    },
    RemoveHook {
        addr: String,
    },
}

#[cw_serde]
pub enum ReceiveMsg {
    Stake {},
    Fund {},
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(StakedBalanceAtHeightResponse)]
    StakedBalanceAtHeight {
        address: String,
        height: Option<u64>,
    },
    #[returns(TotalStakedAtHeightResponse)]
    TotalStakedAtHeight { height: Option<u64> },
    #[returns(StakedValueResponse)]
    StakedValue { address: String },
    #[returns(TotalValueResponse)]
    TotalValue {},
    #[returns(crate::state::Config)]
    GetConfig {},
    #[returns(ClaimsResponse)]
    Claims { address: String },
    #[returns(GetHooksResponse)]
    GetHooks {},
    #[returns(ListStakersResponse)]
    ListStakers {
        start_after: Option<String>,
        limit: Option<u32>,
    },
}

#[cw_serde]
pub enum MigrateMsg {
    FromBeta { manager: Option<String> },
    FromCompatible {},
}

#[cw_serde]
pub struct StakedBalanceAtHeightResponse {
    pub balance: Uint128,
    pub height: u64,
}

#[cw_serde]
pub struct TotalStakedAtHeightResponse {
    pub total: Uint128,
    pub height: u64,
}

#[cw_serde]
pub struct StakedValueResponse {
    pub value: Uint128,
}

#[cw_serde]
pub struct TotalValueResponse {
    pub total: Uint128,
}

#[cw_serde]
pub struct GetHooksResponse {
    pub hooks: Vec<String>,
}

#[cw_serde]
pub struct ListStakersResponse {
    pub stakers: Vec<StakerBalanceResponse>,
}

#[cw_serde]
pub struct StakerBalanceResponse {
    pub address: String,
    pub balance: Uint128,
}
