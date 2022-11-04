use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Uint128};
use cw721::Cw721ReceiveMsg;
use cw_utils::Duration;

use cwd_interface::Admin;
use cwd_macros::{info_query, voting_query};

pub use cw721_controllers::NftClaimsResponse;

use crate::state::Config;

#[cw_serde]
pub struct InstantiateMsg {
    /// May change unstaking duration and add hooks.
    pub owner: Option<Admin>,
    /// Address of the cw721 NFT contract that may be staked.
    pub nft_address: String,
    /// Amount of time between unstaking and tokens being
    /// avaliable. To unstake with no delay, leave as `None`.
    pub unstaking_duration: Option<Duration>,
}

#[cw_serde]
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
#[info_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(StakedBalanceAtHeightResponse)]
    StakedBalanceAtHeight {
        address: String,
        height: Option<u64>,
    },
    #[returns(TotalStakedAtHeightResponse)]
    TotalStakedAtHeight { height: Option<u64> },
    #[returns(Config)]
    GetConfig {},
    #[returns(NftClaimsResponse)]
    NftClaims { address: String },
    #[returns(GetHooksResponse)]
    GetHooks {},
    // List all of the addresses staking with this contract.
    #[returns(Vec<Addr>)]
    ListStakers {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    // List the staked NFTs for a given address.
    #[returns(Vec<String>)]
    StakedNfts {
        address: String,
        start_after: Option<String>,
        limit: Option<u32>,
    },
}

#[cw_serde]
pub struct StakedBalanceAtHeightResponse {
    pub balance: Uint128,
    pub height: u64,
}

#[cw_serde]
pub struct TotalStakedAtHeightResponse {
    pub total: Uint128,
    pub height: u64,
}

#[cw_serde]
pub struct GetHooksResponse {
    pub hooks: Vec<String>,
}

#[cw_serde]
pub struct MigrateMsg {}
