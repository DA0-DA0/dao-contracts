use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::env::current_dir;
use std::fs::create_dir_all;

use cosmwasm_schema::{export_schema, remove_schemas, schema_for};

use cw_governance_macros::cw_governance_voting_query;

#[cw_governance_voting_query]
#[derive(Serialize, Deserialize, JsonSchema)]
enum VotingQuery {}

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema(&schema_for!(VotingQuery), &out_dir);
}
