use cosmwasm_std::Uint128;
use cw20::Cw20ReceiveMsg;
use cw721::Cw721ReceiveMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cw_utils::Duration;

pub use cw_controllers::ClaimsResponse;

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct InstantiateMsg {
    // Owner can update all configs including changing the owner. This will generally be a DAO.
    pub owner: Option<String>,
    // Manager can update all configs except changing the owner. This will generally be an operations multisig for a DAO.
    pub manager: Option<String>,
    pub nft_address: String,
    pub reward_token_address: Option<String>,
    pub unstaking_duration: Option<Duration>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    ReceiveNft(Cw721ReceiveMsg),
    Receive(Cw20ReceiveMsg),
    Unstake {
        token_id: String,
        reward_wallet_address: Option<String>,
    },
    ClaimNfts {},
    ClaimRewards {},
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
    StakedValue {
        address: String,
    },
    TotalValue {},
    GetConfig {},
    NftClaims {
        address: String,
    },
    RewardClaims {
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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct StakedValueResponse {
    pub value: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct TotalValueResponse {
    pub total: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct GetConfigResponse {
    pub owner: Option<String>,
    pub manager: Option<String>,
    pub nft_address: String,
    pub reward_token_address: Option<String>,
    pub unstaking_duration: Option<Duration>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct GetHooksResponse {
    pub hooks: Vec<String>,
}
