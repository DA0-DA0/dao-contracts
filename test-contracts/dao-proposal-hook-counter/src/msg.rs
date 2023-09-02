use cosmwasm_schema::{cw_serde, QueryResponses};
use dao_hooks::{proposal::ProposalHookMsg, stake::StakeChangedHookMsg, vote::VoteHookMsg};

#[cw_serde]
pub struct InstantiateMsg {
    pub should_error: bool, // Debug flag to test when hooks fail over
}

#[cw_serde]
pub enum ExecuteMsg {
    ProposalHook(ProposalHookMsg),
    StakeHook(StakeChangedHookMsg),
    VoteHook(VoteHookMsg),
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(u64)]
    StakeCounter {},
    #[returns(u64)]
    VoteCounter {},
    #[returns(u64)]
    ProposalCounter {},
    #[returns(u64)]
    StatusChangedCounter {},
}

#[cw_serde]
pub struct CountResponse {
    pub count: u64,
}
