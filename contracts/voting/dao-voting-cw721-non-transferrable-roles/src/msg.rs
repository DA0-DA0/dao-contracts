use cosmwasm_schema::{cw_serde, QueryResponses};
use cw721::Cw721ReceiveMsg;
use cw_utils::Duration;
use dao_interface::Admin;
use dao_macros::voting_module_query;

#[cw_serde]
pub struct InstantiateMsg {
    /// May add hooks.
    pub owner: Option<Admin>,
    /// Address of the cw721 NFT contract that may be staked.
    pub nft_address: String,
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
