use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint128;
use cw20::{Cw20ReceiveMsg};

use crate::balance::WrappedBalance;

#[cw_serde]
pub struct InstantiateMsg {
    pub admin: Option<String>,
}
pub type StreamId = u64;


#[cw_serde]
pub enum ExecuteMsg {
    Receive(Cw20ReceiveMsg),
    Distribute {
        id: StreamId, // Stream id
    },
    // TODO: Add this! :D
    // TODO: Only called by stream admin
    // NOTE: Pauses a stream (paused: true), 
    //       vesting is stopped
    //       distribute of unlocked funds should be allowed (based on paused_time)
    PauseStream {
        id: StreamId, // Stream id
    },
    // TODO: Add this! :D
    // TODO: Only called by stream admin
    // NOTE: Removes any pause from a stream, 
    //       optionally can shift the start/end time if needed for vesting flexibility
    ResumeStream {
        id: StreamId, // Stream id
        start_time: Option<u64>,
        end_time: Option<u64>
    },
    // TODO: Add this! :D
    // TODO: Only called by stream admin
    // NOTE: Remove returns funds to the admin
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
    },
}

#[cw_serde]
pub struct StreamParams {
    pub admin: String,
    pub recipient: String,
    pub balance: WrappedBalance,
    pub start_time: u64,
    pub end_time: u64,
    pub paused_time: Option<u64>,
    pub paused: bool,
    pub title: Option<String>,
    pub description: Option<String>,
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
    pub paused: bool,
    pub rate_per_second: Uint128,
    pub title: Option<String>,
    pub description: Option<String>,
}

#[cw_serde]
pub struct ListStreamsResponse {
    pub streams: Vec<StreamResponse>,
}
