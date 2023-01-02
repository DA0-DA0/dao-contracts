use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint128;
use cw20::Cw20ReceiveMsg;
use cw_denom::CheckedDenom;

#[cw_serde]
pub struct InstantiateMsg {
    pub admin: Option<String>,
}

#[cw_serde]
pub enum ExecuteMsg {
    Receive(Cw20ReceiveMsg),
    Create(StreamParams),
    Distribute { id: u64 },
    Pause { id: u64 },
    Resume { id: u64 },
    Cancel { id: u64 },
    Delegate {},
    Undelegate {},
    Redelgate {},
    WithdrawRewards {},
}

// Receiver setup
#[cw_serde]
pub enum ReceiveMsg {
    CreateStream(StreamParams),
}

// TODO rename to vesting
#[cw_serde]
pub struct StreamParams {
    pub recipient: String,
    pub balance: Uint128,
    pub denom: CheckedDenom,
    // pub curve: Curve,
    pub start_time: u64,
    pub end_time: u64,
    pub title: Option<String>,
    pub description: Option<String>,
}

// TODO get stream by recipient
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(ConfigResponse)]
    Config {},
    #[returns(StreamResponse)]
    GetStream { id: u64 },
    #[returns(ListStreamsResponse)]
    ListStreams {
        start: Option<u8>,
        limit: Option<u8>,
    },
}

#[cw_serde]
pub struct ConfigResponse {
    pub admin: String,
}

#[cw_serde]
pub struct StreamResponse {
    pub id: u64,
    pub recipient: String,
    pub balance: Uint128,
    pub claimed_balance: Uint128,
    pub denom: CheckedDenom,
    pub start_time: u64,
    pub end_time: u64,
    pub paused_time: Option<u64>,
    pub paused_duration: Option<u64>,
    /// Whether the payroll stream is currently paused
    pub paused: bool,
    /// Human readable title for this contract
    pub title: Option<String>,
    /// Human readable description for this payroll contract
    pub description: Option<String>,
}

#[cw_serde]
pub struct ListStreamsResponse {
    pub streams: Vec<StreamResponse>,
}
