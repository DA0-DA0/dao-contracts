use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::WasmMsg;

#[cw_serde]
pub struct InstantiateMsg {}

#[cw_serde]
pub enum ExecuteMsg {
    StargazeBaseMinterFactory(WasmMsg),
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(dao_interface::voting::InfoResponse)]
    Info {},
}
