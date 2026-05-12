use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Binary;
use cw721_base::msg::InstantiateMsg as Cw721InstantiateMsg;

#[cfg(feature = "osmosis_tokenfactory")]
use dao_interface::token::NewTokenInfo;

#[cw_serde]
pub struct InstantiateMsg {}

#[cw_serde]
pub enum ExecuteMsg {
    /// Example NFT factory implementation
    NftFactory {
        code_id: u64,
        cw721_instantiate_msg: Cw721InstantiateMsg,
        initial_nfts: Vec<Binary>,
    },
    /// Example NFT factory implentation that execpts funds
    NftFactoryWithFunds {
        code_id: u64,
        cw721_instantiate_msg: Cw721InstantiateMsg,
        initial_nfts: Vec<Binary>,
    },
    /// Used for testing no callback
    NftFactoryNoCallback {},
    /// Used for testing wrong callback
    NftFactoryWrongCallback {},
    /// Example Factory Implementation
    #[cfg(feature = "osmosis_tokenfactory")]
    TokenFactoryFactory(NewTokenInfo),
    /// Example Factory Implementation that accepts funds
    #[cfg(feature = "osmosis_tokenfactory")]
    TokenFactoryFactoryWithFunds(NewTokenInfo),
    /// Used for testing no callback
    #[cfg(feature = "osmosis_tokenfactory")]
    TokenFactoryFactoryNoCallback {},
    /// Used for testing wrong callback
    #[cfg(feature = "osmosis_tokenfactory")]
    TokenFactoryFactoryWrongCallback {},
    /// Validate NFT DAO
    ValidateNftDao {},
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(dao_interface::voting::InfoResponse)]
    Info {},
}
