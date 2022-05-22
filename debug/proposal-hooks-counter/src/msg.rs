use proposal_hooks::ProposalHookMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use vote_hooks::VoteHookMsg;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct InstantiateMsg {
    pub should_error: bool, // Debug flag to test when hooks fail over
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    ProposalHook(ProposalHookMsg),
    VoteHook(VoteHookMsg),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    VoteCounter {},
    ProposalCounter {},
    StatusChangedCounter {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct CountResponse {
    pub count: u64,
}
