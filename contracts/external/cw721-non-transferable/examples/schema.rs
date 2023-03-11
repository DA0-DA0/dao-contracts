use cosmwasm_schema::write_api;
use cosmwasm_std::Empty;

use cw721_base::{ExecuteMsg, InstantiateMsg, QueryMsg};
use cw721_soulbound_roles::{ExecuteExt, MetadataExt, QueryExt};

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        execute: ExecuteMsg<MetadataExt, ExecuteExt>,
        query: QueryMsg<QueryExt>,
    }
}
