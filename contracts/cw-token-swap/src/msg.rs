use cosmwasm_std::Uint128;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::state::CheckedCounterparty;

/// Information about the token being used on one side of the escrow.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TokenInfo {
    /// A native token.
    Native { denom: String, amount: Uint128 },
    /// A cw20 token.
    Cw20 {
        contract_addr: String,
        amount: Uint128,
    },
}

/// Information about a counterparty in this escrow transaction and
/// their promised funds.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct Counterparty {
    /// The address of the counterparty.
    pub address: String,
    /// The funds they have promised to provide.
    pub promise: TokenInfo,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct InstantiateMsg {
    pub counterparty_one: Counterparty,
    pub counterparty_two: Counterparty,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// Used to provide cw20 tokens to satisfy a funds promise.
    Receive(cw20::Cw20ReceiveMsg),
    /// Provides native tokens to satisfy a funds promise.
    Fund {},
    /// Withdraws provided funds. Only allowed if the other
    /// counterparty has yet to provide their promised funds.
    Withdraw {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    // Gets the current status of the escrow transaction. Returns
    // StatusResponse.
    Status {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct StatusResponse {
    pub counterparty_one: CheckedCounterparty,
    pub counterparty_two: CheckedCounterparty,
}
