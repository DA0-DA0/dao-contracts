use std::env::current_dir;
use std::fs::create_dir_all;

use cosmwasm_schema::{export_schema, export_schema_with_title, remove_schemas, schema_for};

use cw_usdc::msg::{
    AllowanceResponse, AllowancesResponse, BlacklisteesResponse, BlacklisterAllowancesResponse,
    DenomResponse, ExecuteMsg, FreezerAllowancesResponse, InstantiateMsg, IsFrozenResponse,
    OwnerResponse, QueryMsg, StatusResponse, SudoMsg,
};
use cw_usdc::state::Config;

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema(&schema_for!(InstantiateMsg), &out_dir);
    export_schema(&schema_for!(ExecuteMsg), &out_dir);
    export_schema(&schema_for!(QueryMsg), &out_dir);
    export_schema(&schema_for!(Config), &out_dir);
    export_schema(&schema_for!(SudoMsg), &out_dir);

    export_schema(&schema_for!(IsFrozenResponse), &out_dir);
    export_schema(&schema_for!(DenomResponse), &out_dir);
    export_schema(&schema_for!(OwnerResponse), &out_dir);

    export_schema_with_title(
        &schema_for!(AllowanceResponse),
        &out_dir,
        "MintAllowanceResponse",
    );

    export_schema_with_title(
        &schema_for!(AllowanceResponse),
        &out_dir,
        "BurnAllowanceResponse",
    );

    export_schema_with_title(
        &schema_for!(AllowancesResponse),
        &out_dir,
        "MintAllowancesResponse",
    );

    export_schema_with_title(
        &schema_for!(AllowancesResponse),
        &out_dir,
        "BurnAllowancesResponse",
    );

    export_schema_with_title(
        &schema_for!(StatusResponse),
        &out_dir,
        "IsBlacklistedResponse",
    );

    export_schema(&schema_for!(BlacklisteesResponse), &out_dir);

    export_schema_with_title(
        &schema_for!(StatusResponse),
        &out_dir,
        "IsBlacklisterResponse",
    );

    export_schema(&schema_for!(BlacklisterAllowancesResponse), &out_dir);

    export_schema(&schema_for!(FreezerAllowancesResponse), &out_dir);

    export_schema_with_title(&schema_for!(StatusResponse), &out_dir, "IsFreezerResponse");
}
