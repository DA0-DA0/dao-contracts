use cosmwasm_schema::write_api;

use cosmwasm_std::Empty;
use marketing_gauge_adapter::msg::{AdapterQueryMsg, InstantiateMsg, MigrateMsg};

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        execute: Empty,
        query: AdapterQueryMsg,
        migrate: MigrateMsg,
    }
}
