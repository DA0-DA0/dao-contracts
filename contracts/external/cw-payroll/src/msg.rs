use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Response;
use cw20::Cw20ReceiveMsg;

use crate::{balance::WrappedBalance, ContractError};

#[cw_serde]
pub struct InstantiateMsg {
    pub admin: Option<String>,
}
pub type StreamId = u64;
pub type ContractResult = Result<Response, ContractError>;

#[cw_serde]
pub enum ExecuteMsg {
    Receive(Cw20ReceiveMsg),
    Distribute {
        id: StreamId, // Stream id
    },
    PauseStream {
        id: StreamId, // Stream id
    },
    LinkStream {
        left_stream_id: StreamId,
        right_stream_id: StreamId,
    },
    DetachStream {
        left_stream_id: StreamId,
        right_stream_id: StreamId,
    },
    ResumeStream {
        id: StreamId, // Stream id
    },
    RemoveStream {
        id: StreamId, // Stream id
    },
}

// Receiver setup
#[cw_serde]
pub enum ReceiveMsg {
    CreateStream {
        admin: Option<String>,
        recipient: String,
        start_time: u64,
        end_time: u64,
        is_detachable: Option<bool>,
    },
}

#[cw_serde]
pub struct StreamParams {
    pub admin: String,
    pub recipient: String,
    pub balance: WrappedBalance,
    pub start_time: u64,
    pub end_time: u64,
    pub title: Option<String>,
    pub description: Option<String>,
    pub is_detachable: Option<bool>,
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(ConfigResponse)]
    GetConfig {},
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
    pub admin: String,
    pub recipient: String,
    pub balance: WrappedBalance,
    pub claimed_balance: WrappedBalance,
    pub start_time: u64,
    pub end_time: u64,
    pub paused_time: Option<u64>,
    pub paused_duration: Option<u64>,
    /// Whether the payroll contract is currently paused
    pub paused: bool,
    /// Human readable title for this contract
    pub title: Option<String>,
    /// Human readable description for this payroll contract
    pub description: Option<String>,
    /// Link to stream attached for sync
    pub link_id: Option<StreamId>,
    /// If Stream is detachable
    pub is_detachable: bool,
}

#[cw_serde]
pub struct ListStreamsResponse {
    pub streams: Vec<StreamResponse>,
}
