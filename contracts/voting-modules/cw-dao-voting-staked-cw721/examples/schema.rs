use cosmwasm_schema::{export_schema, export_schema_with_title, remove_schemas, schema_for};
use cw_dao_core_interface::voting::{
    InfoResponse, TotalPowerAtHeightResponse, VotingPowerAtHeightResponse,
};
use cw_dao_voting_staked_cw721::{
    msg::{
        ExecuteMsg, GetHooksResponse, InstantiateMsg, NftClaimsResponse, QueryMsg,
        StakedBalanceAtHeightResponse, TotalStakedAtHeightResponse,
    },
    state::Config,
};
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
    export_schema_with_title(&schema_for!(Config), &out_dir, "GetConfigResponse");
    export_schema_with_title(&schema_for!(Vec<String>), &out_dir, "StakedNftsResponse");
    export_schema_with_title(&schema_for!(Vec<String>), &out_dir, "ListStakersResponse");
    export_schema(&schema_for!(NftClaimsResponse), &out_dir);
    export_schema(&schema_for!(GetHooksResponse), &out_dir);
    export_schema(&schema_for!(TotalPowerAtHeightResponse), &out_dir);
    export_schema(&schema_for!(VotingPowerAtHeightResponse), &out_dir);
    export_schema(&schema_for!(InfoResponse), &out_dir);
}
