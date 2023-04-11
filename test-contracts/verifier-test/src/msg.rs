use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::CosmosMsg;
use cw_verifier_middleware::msg::WrappedMessage;

#[cw_serde]
pub struct InstantiateMsg {}

#[cw_serde]
pub enum InnerExecuteMsg {
    Execute,
}

#[cw_serde]

pub struct ExecuteMsg {
    pub wrapped_msg: WrappedMessage,
}

#[cw_serde]

pub struct QueryMsg {}
