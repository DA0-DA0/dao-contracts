use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint128;
use cw721::Cw721ReceiveMsg;
pub use cw721_controllers::NftClaimsResponse;
use cw_utils::Duration;
use cwd_interface::Admin;
use cwd_macros::{info_query, voting_query};

#[cw_serde]
pub struct InstantiateMsg {
    // Owner can update all configs including changing the owner. This
    // will generally be a DAO.
    pub owner: Option<Admin>,
    // Manager can update all configs except changing the owner. This
    // will generally be an operations multisig for a DAO.
    pub manager: Option<String>,
    pub nft_address: String,
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
    #[returns(crate::state::Config)]
    GetConfig {},
    #[returns(NftClaimsResponse)]
    NftClaims { address: String },
    #[returns(GetHooksResponse)]
    GetHooks {},
    // List all of the addresses staking with this contract.
    #[returns(Vec<cosmwasm_std::Addr>)]
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
