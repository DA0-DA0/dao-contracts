use cosmwasm_schema::{cw_serde, QueryResponses};
use dao_hooks::{proposal::ProposalHookMsg, vote::VoteHookMsg};

use crate::state::{ProposalIncentives, VotingIncentives};

#[cw_serde]
pub struct InstantiateMsg {
    /// DAO address
    pub dao: String,
    /// Rewards to pay out for successful proposals.
    pub proposal_incentives: ProposalIncentives,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Fires when a new proposal status has changed.
    ProposalHook(ProposalHookMsg),
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Returns the config.
    #[returns(ConfigResponse)]
    Config {},
}

#[cw_serde]
pub struct ConfigResponse {
    /// DAO address
    pub dao: String,
    /// Rewards to pay out for successful proposals.
    pub proposal_incentives: ProposalIncentives,
}

#[cw_serde]
pub struct MigrateMsg {}
