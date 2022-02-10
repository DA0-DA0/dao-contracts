use cosmwasm_std::{Addr, Uint128};
use cw20::Cw20ReceiveMsg;
use cw20::Denom;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub start_block: u64,
    pub end_block: u64,
    pub payment_per_block: Uint128,
    pub total_amount: Uint128,
    pub denom: Denom,
    pub distribution_token: String,
    pub payment_block_delta: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Claim {},
    Receive(Cw20ReceiveMsg),
    ClaimUpToBlock { block: u64 },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReceiveMsg {
    /// Adds all sent native tokens to the contract
    Fund {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    // GetCount returns the current count as a json-encoded number
    Info {},
    ClaimableRewards { address: Addr },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct InfoResponse {
    pub start_block: u64,
    pub end_block: u64,
    pub payment_per_block: Uint128,
    pub total_amount: Uint128,
    pub denom: Denom,
    pub staking_contract: String,
    pub blocks_between_payments: u64,
    pub funded: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct ClaimableRewardsResponse {
    pub amount: Uint128,
}
