use cosmwasm_schema::write_api;
use cw20_hooks::msg::ExecuteMsg;
use cw20_hooks::msg::InstantiateMsg;
use cw20_hooks::msg::QueryMsg;

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        query: QueryMsg,
        execute: ExecuteMsg,
    }
}
