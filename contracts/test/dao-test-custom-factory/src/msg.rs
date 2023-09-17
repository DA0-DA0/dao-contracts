use cosmwasm_schema::{cw_serde, QueryResponses};
use cw721_base::InstantiateMsg as Cw721InstantiateMsg;
use dao_interface::token::NewTokenInfo;

#[cw_serde]
pub struct InstantiateMsg {}

#[cw_serde]
pub enum ExecuteMsg {
    TokenFactoryFactory(NewTokenInfo),
    TokenFactoryFactoryWithFunds(NewTokenInfo),
    TokenFactoryFactoryNoCallback {},
    TokenFactoryFactoryWrongCallback {},
    NftFactory {
        code_id: u64,
        cw721_instantiate_msg: Cw721InstantiateMsg,
    },
    NftFactoryWithFunds {
        code_id: u64,
        cw721_instantiate_msg: Cw721InstantiateMsg,
    },
    NftFactoryNoCallback {},
    NftFactoryWrongCallback {},
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(dao_interface::voting::InfoResponse)]
    Info {},
}
