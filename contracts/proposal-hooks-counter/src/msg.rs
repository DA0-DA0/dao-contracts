use proposal_hooks::ProposalHookMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use vote_hooks::VoteHookMsg;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    ProposalHook(ProposalHookMsg),
    VoteHook(VoteHookMsg),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    VoteCounter {},
    ProposalCounter {},
    StatusChangedCounter {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct CountResponse {
    pub count: u64,
}
