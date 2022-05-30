use std::env::current_dir;
use std::fs::create_dir_all;

use cosmwasm_schema::{export_schema, remove_schemas, schema_for};

use cw_fund_distributor::msg::{
    AdminResponse, Cw20EntitlementResponse, DenomResponse, ExecuteMsg, InstantiateMsg,
    NativeEntitlementResponse, QueryMsg, VotingContractResponse,
};

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema(&schema_for!(InstantiateMsg), &out_dir);
    export_schema(&schema_for!(ExecuteMsg), &out_dir);
    export_schema(&schema_for!(QueryMsg), &out_dir);
    export_schema(&schema_for!(AdminResponse), &out_dir);
    export_schema(&schema_for!(Cw20EntitlementResponse), &out_dir);
    export_schema(&schema_for!(DenomResponse), &out_dir);
    export_schema(&schema_for!(NativeEntitlementResponse), &out_dir);
    export_schema(&schema_for!(VotingContractResponse), &out_dir);
}
