use std::env::current_dir;
use std::fs::create_dir_all;

use cosmwasm_schema::{export_schema, remove_schemas, schema_for};

use cw4::AdminResponse;
use cw4::HooksResponse;
use cw4::MemberListResponse;
use cw4::MemberResponse;
use cw4::TotalWeightResponse;
use cw4_group_sorted::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use cw4_group_sorted::query::Member;

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema(&schema_for!(InstantiateMsg), &out_dir);
    export_schema(&schema_for!(ExecuteMsg), &out_dir);
    export_schema(&schema_for!(QueryMsg), &out_dir);
    export_schema(&schema_for!(Member), &out_dir);

    export_schema(&schema_for!(MemberListResponse), &out_dir);
    export_schema(&schema_for!(MemberResponse), &out_dir);
    export_schema(&schema_for!(TotalWeightResponse), &out_dir);
    export_schema(&schema_for!(HooksResponse), &out_dir);
    export_schema(&schema_for!(AdminResponse), &out_dir);
}
