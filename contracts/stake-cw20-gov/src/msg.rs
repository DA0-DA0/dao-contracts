use cosmwasm_std::{Addr, Uint128};
use cw20::Cw20ReceiveMsg;
use cw_utils::Duration;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
pub use stake_cw20::msg::{
    InstantiateMsg, StakedBalanceAtHeightResponse, TotalStakedAtHeightResponse,
};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Receive(Cw20ReceiveMsg),
    Unstake { amount: Uint128 },
    Claim {},
    DelegateVotes { recipient: String },
    UpdateConfig { admin: Addr },
    UpdateUnstakingDuration { duration: Option<Duration> },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReceiveMsg {
    Stake {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Returns the balance of the given address at given height, 0 if unset.
    /// Return type: BalanceAtHeightResponse.
    VotingPowerAtHeight {
        address: String,
        height: Option<u64>,
    },
    /// Returns current delegation information
    /// Return type: DelegationResponse.
    Delegation { address: String },
    StakedBalanceAtHeight {
        address: String,
        height: Option<u64>,
    },
    /// Returns the total staked amount of tokens at a given height, if no height is provided
    /// defaults to current block height.
    TotalStakedAtHeight { height: Option<u64> },
    /// Returns the unstaking duration for the contract.
    UnstakingDuration {},
    /// Returns existing claims for tokens currently unstaking for a given address.
    Claims { address: String },
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct VotingPowerAtHeightResponse {
    pub balance: Uint128,
    pub height: u64,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct DelegationResponse {
    pub delegation: String,
}
