use cosmwasm_std::Uint128;
use cw721::Cw721ReceiveMsg;
use cw_core_macros::voting_query;
use cw_utils::Duration;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub use cw721_controllers::NftClaimsResponse;

#[derive(Serialize, Deserialize, Clone, JsonSchema, Debug)]
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

#[derive(Serialize, Deserialize, Clone, JsonSchema, Debug)]
pub struct InstantiateMsg {
    // Owner can update all configs including changing the owner. This
    // will generally be a DAO.
    pub owner: Option<Owner>,
    // Manager can update all configs except changing the owner. This
    // will generally be an operations multisig for a DAO.
    pub manager: Option<String>,
    pub nft_address: String,
    pub unstaking_duration: Option<Duration>,
}

#[derive(Serialize, Deserialize, Clone, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    ReceiveNft(Cw721ReceiveMsg),
    /// Unstakes the specified token_ids on behalf of the
    /// sender. token_ids must have unique values and have non-zero
    /// length.
    Unstake {
        token_ids: Vec<String>,
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
#[derive(Serialize, Deserialize, Clone, JsonSchema, Debug)]
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
    // List all of the addresses staking with this contract.
    ListStakers {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    // List the staked NFTs for a given address.
    StakedNfts {
        address: String,
        start_after: Option<String>,
        limit: Option<u32>,
    },
}

#[derive(Serialize, Deserialize, Clone, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub struct StakedBalanceAtHeightResponse {
    pub balance: Uint128,
    pub height: u64,
}

#[derive(Serialize, Deserialize, Clone, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub struct TotalStakedAtHeightResponse {
    pub total: Uint128,
    pub height: u64,
}

#[derive(Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct GetHooksResponse {
    pub hooks: Vec<String>,
}
