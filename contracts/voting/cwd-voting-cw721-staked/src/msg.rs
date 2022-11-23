use cosmwasm_schema::{cw_serde, QueryResponses};
use cw721::Cw721ReceiveMsg;
use cw_utils::Duration;
use cwd_interface::Admin;
use cwd_macros::voting_module_query;

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
    /// Used to stake NFTs. To stake a NFT send a cw721 send message
    /// to this contract with the NFT you would like to stake. The
    /// `msg` field is ignored.
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

#[voting_module_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(crate::state::Config)]
    Config {},
    #[returns(::cw721_controllers::NftClaimsResponse)]
    NftClaims { address: String },
    #[returns(::cw_controllers::HooksResponse)]
    Hooks {},
    // List the staked NFTs for a given address.
    #[returns(Vec<String>)]
    StakedNfts {
        address: String,
        start_after: Option<String>,
        limit: Option<u32>,
    },
}
