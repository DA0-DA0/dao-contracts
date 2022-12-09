use cosmwasm_schema::write_api;
use dao_voting_cw4::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, MigrateMsg};

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        query: QueryMsg,
        execute: ExecuteMsg,
        migrate: MigrateMsg,
    }
}
