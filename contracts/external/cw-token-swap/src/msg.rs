use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint128;

use crate::types::{CheckedCounterparty, Counterparty, SendMessage};

#[cw_serde]
pub struct InstantiateMsg {
    pub counterparty_one: Counterparty,
    pub counterparty_two: Counterparty,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Used to provide cw20 tokens to satisfy a funds promise.
    Receive(cw20::Cw20ReceiveMsg),
    /// Provides native tokens to satisfy a funds promise.
    Fund { send_message: Option<SendMessage> },
    /// Withdraws provided funds. Only allowed if the other
    /// counterparty has yet to provide their promised funds.
    Withdraw {},
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    // Gets the current status of the escrow transaction.
    #[returns(crate::msg::StatusResponse)]
    Status {},
}

#[cw_serde]
pub struct StatusResponse {
    pub counterparty_one: CheckedCounterparty,
    pub counterparty_two: CheckedCounterparty,
}

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
pub enum Cw20RecieveMsg {
    FundWithMsgs {
        amount: Uint128,
        send_message: SendMessage,
    },
}
