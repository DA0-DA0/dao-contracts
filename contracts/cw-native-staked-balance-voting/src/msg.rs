use cosmwasm_std::Uint128;
use cw_core_macros::voting_query;
use cw_utils::Duration;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum Owner {
    /// Set the owner to a specific address.
    Addr(String),
    /// Set the owner to the address that instantiates this
    /// contract. This is useful for DAOs that instantiate this
    /// contract as part of their creation process and would like to
    /// set themselces as the admin.
    Instantiator {},
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct InstantiateMsg {
    // Owner can update all configs including changing the owner. This will generally be a DAO.
    pub owner: Option<Owner>,
    // Manager can update all configs except changing the owner. This will generally be an operations multisig for a DAO.
    pub manager: Option<String>,
    // Token denom e.g. ujuno, or some ibc denom
    pub denom: String,
    // How long until the tokens become liquid again
    pub unstaking_duration: Option<Duration>,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Stake {},
    Unstake {
        amount: Uint128,
    },
    UpdateConfig {
        owner: Option<String>,
        manager: Option<String>,
        duration: Option<Duration>,
    },
    Claim {},
}

#[voting_query]
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Dao {},
    GetConfig {},
    Claims {
        address: String,
    },
    ListStakers {
        start_after: Option<String>,
        limit: Option<u32>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct MigrateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct ListStakersResponse {
    pub stakers: Vec<StakerBalanceResponse>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct StakerBalanceResponse {
    pub address: String,
    pub balance: Uint128,
}
