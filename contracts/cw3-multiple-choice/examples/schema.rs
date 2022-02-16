use std::env::current_dir;
use std::fs::create_dir_all;

use cosmwasm_schema::{export_schema, remove_schemas, schema_for, export_schema_with_title};

use cw3_multiple_choice::{msg::{ExecuteMsg, InstantiateMsg, QueryMsg}, state::{Config, Proposal, VoteInfo}, query::{ProposalResponse, ProposalListResponse, VoteResponse, VoteListResponse, VoteTallyResponse}};

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema(&schema_for!(InstantiateMsg), &out_dir);
    export_schema_with_title(&schema_for!(ExecuteMsg), &out_dir, "ExecuteMsg");
    export_schema_with_title(&schema_for!(QueryMsg), &out_dir, "QueryMsg");

    export_schema_with_title(&schema_for!(Config), &out_dir, "Config");
    export_schema_with_title(&schema_for!(Proposal), &out_dir, "Proposal");
    export_schema_with_title(&schema_for!(ProposalResponse), &out_dir, "ProposalResponse");
    export_schema_with_title(
        &schema_for!(ProposalListResponse),
        &out_dir,
        "ProposalListResponse",
    );
    export_schema_with_title(&schema_for!(VoteInfo), &out_dir, "VoteInfo");
    export_schema_with_title(&schema_for!(VoteResponse), &out_dir, "VoteResponse");
    export_schema_with_title(&schema_for!(VoteListResponse), &out_dir, "VoteListResponse");
    export_schema_with_title(
        &schema_for!(VoteTallyResponse),
        &out_dir,
        "VoteTallyResponse",
    );
}
