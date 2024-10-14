use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal, Uint128};
use dao_interface::voting::InfoResponse;

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Returns contract version info
    #[returns(InfoResponse)]
    Info {},
    /// Returns the paginated list of active delegates.
    #[returns(DelegatesResponse)]
    Delegates {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// Returns the delegations by a delegator, optionally at a given height.
    /// Uses the current block height if not provided.
    #[returns(DelegationsResponse)]
    Delegations {
        delegator: String,
        height: Option<u64>,
        offset: Option<u64>,
        limit: Option<u64>,
    },
    /// Returns the VP delegated to a delegate that has not yet been used in
    /// votes cast by delegators in a specific proposal.
    #[returns(Uint128)]
    UnvotedDelegatedVotingPower {
        delegate: String,
        proposal_module: String,
        proposal_id: u64,
        height: u64,
    },
    /// Returns the proposal modules synced from the DAO.
    #[returns(Vec<Addr>)]
    ProposalModules {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// Returns the voting power hook callers.
    #[returns(Vec<Addr>)]
    VotingPowerHookCallers {
        start_after: Option<String>,
        limit: Option<u32>,
    },
}

#[cw_serde]
pub struct DelegatesResponse {
    /// The delegates.
    pub delegates: Vec<DelegateResponse>,
}

#[cw_serde]
pub struct DelegateResponse {
    /// The delegate.
    pub delegate: Addr,
    /// The total voting power delegated to the delegate.
    pub power: Uint128,
}

#[cw_serde]
#[derive(Default)]
pub struct DelegationsResponse {
    /// The delegations.
    pub delegations: Vec<Delegation>,
    /// The height at which the delegations were loaded.
    pub height: u64,
}

#[cw_serde]
pub struct Delegate {}

#[cw_serde]
pub struct Delegation {
    /// the delegate that can vote on behalf of the delegator.
    pub delegate: Addr,
    /// the percent of the delegator's voting power that is delegated to the
    /// delegate.
    pub percent: Decimal,
}

/// Calculate delegated voting power given a member's total voting power and a
/// percent delegated.
pub fn calculate_delegated_vp(vp: Uint128, percent: Decimal) -> Uint128 {
    if percent.is_zero() || vp.is_zero() {
        return Uint128::zero();
    }

    vp.mul_floor(percent)
}
