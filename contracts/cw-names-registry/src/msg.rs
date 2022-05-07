use cosmwasm_std::{Addr, Uint128};
use cw20::Cw20ReceiveMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub admin: String,
    pub payment_token_address: String,
    pub payment_amount: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Receive(Cw20ReceiveMsg), // Receive payment to register a name
    UpdateConfig {
        new_payment_token_address: Option<String>,
        new_admin: Option<String>,
        new_payment_amount: Option<Uint128>,
    },
    /// Reserve a name so it cannot be taken for later use
    Reserve {
        name: String,
    },
    /// Transfer a reserved name to a DAO
    TransferReservation {
        name: String,
        dao: String,
    },
    Revoke {
        name: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReceiveMsg {
    /// DAO can register a name by paying, we assume DAO is the sender
    Register { name: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Config {},
    LookUpDao { dao: String },
    LookUpName { name: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct LookUpDaoResponse {
    pub name: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct LookUpNameResponse {
    pub reserved: bool, // Reserved is a special case where there is no DAO but it still can't be used
    pub dao: Option<Addr>,
}
