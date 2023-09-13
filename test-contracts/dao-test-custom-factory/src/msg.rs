use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::WasmMsg;
use dao_voting_token_staked::msg::NewTokenInfo;

#[cw_serde]
pub struct InstantiateMsg {}

#[cw_serde]
pub enum ExecuteMsg {
    StargazeBaseMinterFactory(WasmMsg),
    TokenFactoryFactory(NewTokenInfo),
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(dao_interface::voting::InfoResponse)]
    Info {},
}
