use std::env::current_dir;
use std::fs::create_dir_all;

use cosmwasm_schema::{export_schema, export_schema_with_title, remove_schemas, schema_for};
use cosmwasm_std::Addr;
use cw20_vest::{msg::{
    ExecuteMsg, InstantiateMsg, QueryMsg, MigrateMsg, GetFundingStatusAtHeightResponse, GetVestingStatusAtHeightResponse,
}, state::Config};

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema(&schema_for!(InstantiateMsg), &out_dir);
    export_schema(&schema_for!(ExecuteMsg), &out_dir);
    export_schema(&schema_for!(QueryMsg), &out_dir);
    export_schema(&schema_for!(MigrateMsg), &out_dir);

    export_schema(&schema_for!(GetFundingStatusAtHeightResponse), &out_dir);
    export_schema(&schema_for!(GetVestingStatusAtHeightResponse), &out_dir);

    export_schema_with_title(&schema_for!(Config), &out_dir, "GetConfigResponse");
}
