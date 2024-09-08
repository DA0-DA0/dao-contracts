use cosmwasm_schema::cw_serde;
use cosmwasm_std::CosmosMsg;

#[cw_serde]
pub enum ExecuteMsg {
    Execute { msgs: Vec<CosmosMsg> },
}
