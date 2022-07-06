use cosmwasm_schema::{export_schema, remove_schemas, schema_for};
use cw721_stake::msg::{
    ExecuteMsg, GetConfigResponse, GetHooksResponse, InstantiateMsg, NftClaimsResponse, QueryMsg,
    StakedBalanceAtHeightResponse, TotalStakedAtHeightResponse,
};
use cw_core_interface::voting::InfoResponse;
use cw_core_interface::voting::TotalPowerAtHeightResponse;
use cw_core_interface::voting::VotingPowerAtHeightResponse;
use std::env::current_dir;
use std::fs::create_dir_all;

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema(&schema_for!(InstantiateMsg), &out_dir);
    export_schema(&schema_for!(ExecuteMsg), &out_dir);
    export_schema(&schema_for!(QueryMsg), &out_dir);
    export_schema(&schema_for!(StakedBalanceAtHeightResponse), &out_dir);
    export_schema(&schema_for!(TotalStakedAtHeightResponse), &out_dir);
    export_schema(&schema_for!(GetConfigResponse), &out_dir);
    export_schema(&schema_for!(NftClaimsResponse), &out_dir);
    export_schema(&schema_for!(GetHooksResponse), &out_dir);
    export_schema(&schema_for!(TotalPowerAtHeightResponse), &out_dir);
    export_schema(&schema_for!(VotingPowerAtHeightResponse), &out_dir);
    export_schema(&schema_for!(InfoResponse), &out_dir);
}
