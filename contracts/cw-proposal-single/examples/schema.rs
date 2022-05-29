use std::env::current_dir;
use std::fs::create_dir_all;

use cosmwasm_schema::{export_schema, export_schema_with_title, remove_schemas, schema_for};
use cosmwasm_std::Addr;
use cw_core_interface::voting::InfoResponse;
use cw_proposal_single::{
    msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg},
    query::{ProposalListResponse, ProposalResponse, VoteListResponse, VoteResponse},
    state::Config,
};
use indexable_hooks::HooksResponse;

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema(&schema_for!(InstantiateMsg), &out_dir);
    export_schema(&schema_for!(ExecuteMsg), &out_dir);
    export_schema(&schema_for!(QueryMsg), &out_dir);
    export_schema(&schema_for!(MigrateMsg), &out_dir);

    export_schema(&schema_for!(InfoResponse), &out_dir);
    export_schema(&schema_for!(ProposalResponse), &out_dir);
    export_schema(&schema_for!(VoteResponse), &out_dir);

    // Auto TS code generation expects the query return type as QueryNameResponse
    // Here we map query resonses to the correct name
    export_schema_with_title(&schema_for!(Config), &out_dir, "ConfigResponse");
    export_schema_with_title(
        &schema_for!(Vec<Addr>),
        &out_dir,
        "GovernanceModulesResponse",
    );
    export_schema_with_title(
        &schema_for!(ProposalListResponse),
        &out_dir,
        "ListProposalsResponse",
    );
    export_schema_with_title(
        &schema_for!(VoteListResponse),
        &out_dir,
        "ListVotesResponse",
    );
    export_schema_with_title(&schema_for!(u64), &out_dir, "ProposalCountResponse");
    export_schema_with_title(
        &schema_for!(ProposalListResponse),
        &out_dir,
        "ReverseProposalsResponse",
    );
    export_schema_with_title(
        &schema_for!(HooksResponse),
        &out_dir,
        "ProposalHooksResponse",
    );
    export_schema_with_title(&schema_for!(HooksResponse), &out_dir, "VoteHooksResponse");
}
