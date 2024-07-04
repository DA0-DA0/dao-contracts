use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, CosmosMsg, Decimal};
use cw20::Cw20ReceiveMsg;

use crate::state::Asset;

#[cw_serde]
pub struct InstantiateMsg {
    /// Address that is allowed to return deposits.
    pub admin: String,
    /// Deposit required for valid submission. This option allows to reduce spam.
    pub required_deposit: Option<Asset>,
    /// Address of contract where each deposit is transferred.
    pub community_pool: String,
    /// Total reward amount.
    pub reward: Asset,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Implements the Cw20 receiver interface.
    Receive(Cw20ReceiveMsg),
    /// Save info about team that wants to participate.
    /// Only for native tokens as required deposit.
    CreateSubmission {
        name: String,
        url: String,
        address: String,
    },
    /// Sends back all deposit to senders.
    ReturnDeposits {},
}

#[cw_serde]
pub enum ReceiveMsg {
    /// Save info about team that wants to participate.
    /// Only for CW20 tokens as required deposit.
    CreateSubmission {
        name: String,
        url: String,
        address: String,
    },
}

#[cw_serde]
pub enum MigrateMsg {}

// Queries copied from gauge-orchestrator for now (we could use a common crate for this).
/// Queries the gauge requires from the adapter contract in order to function.
#[cw_serde]
#[derive(QueryResponses)]
pub enum AdapterQueryMsg {
    #[returns(crate::state::Config)]
    Config {},
    #[returns(AllOptionsResponse)]
    AllOptions {},
    #[returns(CheckOptionResponse)]
    CheckOption { option: String },
    #[returns(SampleGaugeMsgsResponse)]
    SampleGaugeMsgs {
        /// Option along with weight.
        /// Sum of all weights should be 1.0 (within rounding error).
        selected: Vec<(String, Decimal)>,
    },

    // Marketing-gauge specific queries to help on frontend
    #[returns(SubmissionResponse)]
    Submission { address: String },
    #[returns(AllSubmissionsResponse)]
    AllSubmissions {},
}

#[cw_serde]
pub struct AllOptionsResponse {
    pub options: Vec<String>,
}

#[cw_serde]
pub struct CheckOptionResponse {
    pub valid: bool,
}

#[cw_serde]
pub struct SampleGaugeMsgsResponse {
    pub execute: Vec<CosmosMsg>,
}

#[cw_serde]
pub struct SubmissionResponse {
    pub sender: Addr,
    pub name: String,
    pub url: String,
    pub address: Addr,
}

#[cw_serde]
pub struct AllSubmissionsResponse {
    pub submissions: Vec<SubmissionResponse>,
}
