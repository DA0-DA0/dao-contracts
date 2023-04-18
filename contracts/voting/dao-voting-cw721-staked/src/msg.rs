use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Empty;
use cw721::Cw721ReceiveMsg;
use cw_utils::Duration;
use dao_interface::Admin;
use dao_macros::{active_query, voting_module_query};
use dao_voting::threshold::ActiveThreshold;

#[cw_serde]
pub struct NftMintMsg {
    /// Unique ID of the NFT
    pub token_id: String,
    /// The owner of the newly minter NFT
    pub owner: String,
    /// Universal resource identifier for this NFT
    /// Should point to a JSON file that conforms to the ERC721
    /// Metadata JSON Schema
    pub token_uri: Option<String>,
    /// Any custom extension used by this contract
    pub extension: Empty,
}

#[cw_serde]
#[allow(clippy::large_enum_variant)]
pub enum NftContract {
    Existing {
        /// Address of an already instantiated cw721 token contract.
        address: String,
    },
    New {
        /// Code ID for cw721 token contract.
        code_id: u64,
        /// Label to use for instantiated cw721 contract.
        label: String,
        name: String,
        symbol: String,
        /// Initial NFTs to mint when creating the NFT contract.
        initial_nfts: Vec<NftMintMsg>,
    },
}

#[cw_serde]
pub struct InstantiateMsg {
    /// May change unstaking duration and add hooks.
    pub owner: Option<Admin>,
    /// Address of the cw721 NFT contract that may be staked.
    pub nft_contract: NftContract,
    /// Amount of time between unstaking and tokens being
    /// avaliable. To unstake with no delay, leave as `None`.
    pub unstaking_duration: Option<Duration>,
    /// The number or percentage of tokens that must be staked
    /// for the DAO to be active
    pub active_threshold: Option<ActiveThreshold>,
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
    /// Sets the active threshold to a new value. Only the
    /// instantiator this contract (a DAO most likely) may call this
    /// method.
    UpdateActiveThreshold {
        new_threshold: Option<ActiveThreshold>,
    },
}

#[active_query]
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
    #[returns(ActiveThresholdResponse)]
    ActiveThreshold {},
}

#[cw_serde]
pub struct ActiveThresholdResponse {
    pub active_threshold: Option<ActiveThreshold>,
}

#[cw_serde]
pub struct MigrateMsg {}
