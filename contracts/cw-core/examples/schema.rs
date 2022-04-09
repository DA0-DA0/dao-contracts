use std::env::current_dir;
use std::fs::create_dir_all;

use cosmwasm_schema::{export_schema, remove_schemas, schema_for};

use cw_core::{
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
    query::{Cw20BalanceResponse, DumpStateResponse, GetItemResponse},
};

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema(&schema_for!(InstantiateMsg), &out_dir);
    export_schema(&schema_for!(ExecuteMsg), &out_dir);
    export_schema(&schema_for!(QueryMsg), &out_dir);

    export_schema(&schema_for!(DumpStateResponse), &out_dir);
    export_schema(&schema_for!(GetItemResponse), &out_dir);
    export_schema(&schema_for!(Cw20BalanceResponse), &out_dir);
}
