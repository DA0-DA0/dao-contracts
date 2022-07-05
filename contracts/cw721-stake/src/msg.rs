use cosmwasm_std::Uint128;
use cw721::Cw721ReceiveMsg;
use cw_core_macros::voting_query;
use cw_utils::Duration;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub use cw721_controllers::NftClaimsResponse;

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct InstantiateMsg {
    // Owner can update all configs including changing the owner. This
    // will generally be a DAO.
    pub owner: Option<String>,
    // Manager can update all configs except changing the owner. This
    // will generally be an operations multisig for a DAO.
    pub manager: Option<String>,
    pub nft_address: String,
    pub unstaking_duration: Option<Duration>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    ReceiveNft(Cw721ReceiveMsg),
    Unstake {
        token_id: String,
    },
    ClaimNfts {},
    UpdateConfig {
        owner: Option<String>,
        manager: Option<String>,
        duration: Option<Duration>,
    },
    AddHook {
        addr: String,
    },
    RemoveHook {
        addr: String,
    },
}

#[voting_query]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    StakedBalanceAtHeight {
        address: String,
        height: Option<u64>,
    },
    TotalStakedAtHeight {
        height: Option<u64>,
    },
    GetConfig {},
    NftClaims {
        address: String,
    },
    GetHooks {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct StakedBalanceAtHeightResponse {
    pub balance: Uint128,
    pub height: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct TotalStakedAtHeightResponse {
    pub total: Uint128,
    pub height: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct GetConfigResponse {
    pub owner: Option<String>,
    pub manager: Option<String>,
    pub nft_address: String,
    pub unstaking_duration: Option<Duration>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct GetHooksResponse {
    pub hooks: Vec<String>,
}
