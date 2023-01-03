use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint128;
use cw20::Cw20ReceiveMsg;
use cw_denom::CheckedDenom;

use crate::state::{StreamId, StreamIds};

#[cw_serde]
pub struct InstantiateMsg {
    pub admin: Option<String>,
}

#[cw_serde]
pub enum ExecuteMsg {
    Receive(Cw20ReceiveMsg),
    // TODO be able to create steam with native token
    Create {
        params: StreamParams,
    },
    Distribute {
        id: StreamId, // Stream id
    },
    PauseStream {
        id: StreamId, // Stream id
    },
    LinkStream {
        ids: StreamIds,
    },
    DetachStream {
        id: StreamId, // Stream id
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
    // TODO support all StreamParams or delete them
    CreateStream {
        owner: Option<String>,
        recipient: String,
        start_time: u64,
        end_time: u64,
        // TODO just make this a bool
        is_detachable: Option<bool>,
    },
}

#[cw_serde]
pub struct StreamParams {
    pub owner: String,
    pub recipient: String,
    pub balance: Uint128,
    pub denom: CheckedDenom,
    pub start_time: u64,
    pub end_time: u64,
    pub title: Option<String>,
    pub description: Option<String>,
    // TODO just make this a bool
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
    pub owner: String,
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
    /// Link to stream attached for sync
    pub link_id: Option<StreamId>,
    /// Making a stream detachable will only affect linked streams.
    /// A linked stream that detaches in the future will pause both streams.
    /// Each stream must then resume on their own, or be fully removed to re-link.
    pub is_detachable: bool,
}

#[cw_serde]
pub struct ListStreamsResponse {
    pub streams: Vec<StreamResponse>,
}
