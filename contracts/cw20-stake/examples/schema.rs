use std::env::current_dir;
use std::fs::create_dir_all;

use cosmwasm_schema::{export_schema, remove_schemas, schema_for};

use cw20::{
    AllAccountsResponse, AllAllowancesResponse, AllowanceResponse, BalanceResponse,
    TokenInfoResponse,
};
use cw20_stake::msg::{
    ClaimsResponse, ExecuteMsg, GetConfigResponse, GetHooksResponse, InstantiateMsg, QueryMsg,
    StakedBalanceAtHeightResponse, StakedValueResponse, TotalStakedAtHeightResponse,
    TotalValueResponse,
};

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
    export_schema(&schema_for!(StakedValueResponse), &out_dir);
    export_schema(&schema_for!(TotalValueResponse), &out_dir);
    export_schema(&schema_for!(GetConfigResponse), &out_dir);
    export_schema(&schema_for!(GetHooksResponse), &out_dir);
    export_schema(&schema_for!(ClaimsResponse), &out_dir);
    export_schema(&schema_for!(AllowanceResponse), &out_dir);
    export_schema(&schema_for!(BalanceResponse), &out_dir);
    export_schema(&schema_for!(TokenInfoResponse), &out_dir);
    export_schema(&schema_for!(AllAllowancesResponse), &out_dir);
    export_schema(&schema_for!(AllAccountsResponse), &out_dir);
}
