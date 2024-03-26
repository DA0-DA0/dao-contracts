use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Binary, Uint128};

#[cw_serde]
pub enum Cw20HookMsg {
    Transfer {
        sender: String,
        recipient: String,
        amount: Uint128,
    },
    Send {
        sender: String,
        amount: Uint128,
        contract: String,
        msg: Binary,
    },
}

#[cw_serde]
pub enum Cw20HookExecuteMsg {
    Cw20Hook(Cw20HookMsg),
}
