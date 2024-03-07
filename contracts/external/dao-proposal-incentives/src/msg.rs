use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint128;
use cw20::Cw20ReceiveMsg;
use cw_denom::UncheckedDenom;
use cw_ownable::cw_ownable_query;
use dao_hooks::proposal::ProposalHookMsg;

use crate::state::ProposalIncentives;

#[cw_serde]
pub struct InstantiateMsg {
    /// The contract's owner using cw-ownable
    pub owner: String,
    /// Rewards to pay out for successful proposals.
    pub proposal_incentives: ProposalIncentivesUnchecked,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Fires when a new proposal status has changed.
    ProposalHook(ProposalHookMsg),
    UpdateOwnership(cw_ownable::Action),
    UpdateProposalIncentives {
        proposal_incentives: ProposalIncentivesUnchecked,
    },
    Receive(Cw20ReceiveMsg),
}

#[cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Returns the proposal incentives
    #[returns(ProposalIncentives)]
    ProposalIncentives { height: Option<u64> },
}

#[cw_serde]
pub struct ProposalIncentivesUnchecked {
    pub rewards_per_proposal: Uint128,
    pub denom: UncheckedDenom,
}

#[cw_serde]
pub enum MigrateMsg {
    FromCompatible {},
}
