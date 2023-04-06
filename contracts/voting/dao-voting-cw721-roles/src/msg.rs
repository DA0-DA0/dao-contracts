use cosmwasm_schema::{cw_serde, QueryResponses};
use dao_interface::Admin;
use dao_macros::voting_module_query;

#[cw_serde]
pub enum NftContract {
    Existing {
        /// Address of an already instantiated cw721-weighted-roles token contract.
        address: String,
    },
    New {
        /// Code ID for cw721 token contract.
        code_id: u64,
        /// Label to use for instantiated cw721 contract.
        label: String,
        name: String,
        symbol: String,
    },
}

#[cw_serde]
pub struct InstantiateMsg {
    /// May add hooks.
    pub owner: Option<Admin>,
    /// Info about the associated NFT contract
    pub nft_contract: NftContract,
}

#[cw_serde]
pub enum ExecuteMsg {
    UpdateConfig { owner: Option<String> },
    AddHook { addr: String },
    RemoveHook { addr: String },
}

#[voting_module_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(crate::state::Config)]
    Config {},
    #[returns(::cw_controllers::HooksResponse)]
    Hooks {},
}
