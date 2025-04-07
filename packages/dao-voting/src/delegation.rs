use cosmwasm_schema::{
    cw_serde,
    serde::{de::DeserializeOwned, Serialize},
    QueryResponses,
};
use cosmwasm_std::{Addr, Decimal, DepsMut, StdResult, Uint128};
use cw_storage_plus::Map;
use dao_interface::voting::InfoResponse;

use crate::proposal::Ballot;

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Returns contract version info
    #[returns(InfoResponse)]
    Info {},
    /// Returns registration info for a delegate, optionally at a given height.
    #[returns(RegistrationResponse)]
    Registration {
        delegate: String,
        height: Option<u64>,
    },
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
    /// votes cast by delegators in a specific proposal. This updates
    /// immediately via vote hooks (instead of being delayed 1 block like other
    /// historical queries), making it safe to vote multiple times in the same
    /// block. Proposal modules are responsible for maintaining the effective VP
    /// cap when a delegator overrides a delegate's vote. The `proposal_height`
    /// field is the height at which the proposal was created.
    #[returns(UnvotedDelegatedVotingPowerResponse)]
    UnvotedDelegatedVotingPower {
        delegate: String,
        proposal_module: String,
        proposal_id: u64,
        proposal_height: u64,
    },
    /// Returns the VP that should be removed from a delegate's effective UDVP
    /// (and vote tally if already voted) on a specific proposal when a
    /// delegator casts a vote (potentially overriding the delegate's vote). The
    /// `proposal_height` field is the height at which the proposal was created.
    /// The `delegated_vp` field is the amount of VP delegated by the delegator
    /// to the delegate for this proposal. This query takes into account the
    /// configured VP cap and should be used by proposal modules when a
    /// delegator overrides a delegate's vote to compute ballot VP updates.
    #[returns(Uint128)]
    EffectiveUnvotedDelegatedVotingPowerReduction {
        proposal_module: String,
        proposal_id: u64,
        proposal_height: u64,
        delegate: String,
        delegated_vp: Uint128,
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
    /// Returns the config.
    #[returns(Config)]
    Config {},
    /// Returns the voting power cap, optionally at a given height.
    #[returns(VotingPowerCapResponse)]
    VotingPowerCap { height: Option<u64> },
}

#[cw_serde]
pub struct RegistrationResponse {
    /// Whether or not the delegate is registered.
    pub registered: bool,
    /// The total voting power delegated to the delegate. If not registered,
    /// this may still be nonzero if the delegate was registered in the past.
    pub power: Uint128,
    /// The height at which registration was checked.
    pub height: u64,
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
    pub delegations: Vec<DelegationResponse>,
    /// The height at which the delegations were loaded.
    pub height: u64,
}

#[cw_serde]
pub struct DelegationResponse {
    /// the delegate that can vote on behalf of the delegator.
    pub delegate: Addr,
    /// the percent of the delegator's voting power that is delegated to the
    /// delegate.
    pub percent: Decimal,
    /// whether or not the delegation is active (i.e. the delegate is still
    /// registered at the corresponding block). this can only be false if the
    /// delegate was registered when the delegation was created and isn't
    /// anymore.
    pub active: bool,
}

#[cw_serde]
#[derive(Default)]
pub struct UnvotedDelegatedVotingPowerResponse {
    /// The total unvoted delegated voting power.
    pub total: Uint128,
    /// The unvoted delegated voting power in effect, with configured
    /// constraints applied, such as the VP cap.
    pub effective: Uint128,
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

#[cw_serde]
pub struct Config {
    /// the number of blocks a delegation is valid for, after which it must be
    /// renewed by the delegator. if not set, the delegation will never expire.
    pub delegation_validity_blocks: Option<u64>,
    /// the total number of delegations a member can have. this should be set
    /// based on the max gas allowed in a single block for the given chain.
    ///
    /// this limit is relevant for two reasons:
    ///  1. when voting power is updated for a delegator, we must loop through
    ///     all of their delegates and update their delegated voting power
    ///  2. when a delegator casts a vote on a proposal that overrides their
    ///     delegates' votes, we must loop through all of their delegates and
    ///     update the proposal vote tally accordingly
    ///
    /// in tests on Neutron, with a block max gas of 30M (which is one of the
    /// lowest gas limits on any chain), we found that 50 delegations is a safe
    /// upper bound.
    pub max_delegations: u64,
}

#[cw_serde]
pub struct VotingPowerCapResponse {
    /// The voting power cap percent.
    pub vp_cap_percent: Option<Decimal>,
    /// The height at which the voting power cap was loaded.
    pub height: u64,
}

/// Calculate delegated voting power given a member's total voting power and a
/// percent delegated.
pub fn calculate_delegated_vp(vp: Uint128, percent: Decimal) -> Uint128 {
    if percent.is_zero() || vp.is_zero() {
        return Uint128::zero();
    }

    vp.mul_floor(percent)
}

// DELEGATE VOTE OVERRIDE: if this is the first time this member voted, override
// their delegates' votes with the delegator's vote.
//
// subtract the delegator's respective delegated VP  amounts from the vote tally
// of all of their delegates who already voted on this proposal in order to
// override their vote with the delegator's preference.
//
// we must load all delegations and update each. if this partially fails, the
// vote tallies will be incorrect, so the entire vote transaction should fail.
// we need to prevent this from running out of gas by limiting the number of
// delegations a member can have in order to ensure votes can always be cast.
#[allow(clippy::too_many_arguments)]
pub fn handle_delegate_vote_override<Vote: Serialize + DeserializeOwned>(
    deps: DepsMut,
    delegator: &Addr,
    delegation_module: &Option<Addr>,
    proposal_module: &Addr,
    proposal_id: u64,
    proposal_height: u64,
    individual_vote_power: &Uint128,
    ballots: Map<(u64, &Addr), Ballot<Vote>>,
    remove_vote: &mut impl FnMut(&Vote, Uint128) -> StdResult<()>,
) -> StdResult<()> {
    if let Some(delegation_module) = delegation_module {
        let delegations = deps
            .querier
            .query_wasm_smart::<DelegationsResponse>(
                delegation_module,
                &QueryMsg::Delegations {
                    delegator: delegator.to_string(),
                    height: Some(proposal_height),
                    offset: None,
                    limit: None,
                },
                // ensure query error gets returned if it fails.
            )?
            .delegations;

        for DelegationResponse {
            delegate,
            percent,
            active,
        } in delegations
        {
            // if delegation is not active, skip.
            if !active {
                continue;
            }

            // if delegate voted already, untally the VP the delegator delegated
            // to them since the delegate's vote is being overridden.
            if let Some(mut delegate_ballot) =
                ballots.may_load(deps.storage, (proposal_id, &delegate))?
            {
                let delegated_vp = calculate_delegated_vp(*individual_vote_power, percent);

                // get the amount of VP the delegate should lose due to this
                // delegator's vote override. this loss should be equal to the
                // delegated VP or less if the delegated VP is already being
                // capped due to the delegation module config.
                let reduction: Uint128 = deps.querier.query_wasm_smart(
                    delegation_module,
                    &QueryMsg::EffectiveUnvotedDelegatedVotingPowerReduction {
                        proposal_module: proposal_module.to_string(),
                        proposal_id,
                        proposal_height,
                        delegate: delegate.to_string(),
                        delegated_vp,
                    },
                )?;

                // if the delegate's effective VP must be reduced, update ballot
                // total and vote tally. this diff method makes sure to preserve
                // the delegate's individual VP even if they lose all delegated
                // VP due to delegators overriding votes.
                if !reduction.is_zero() {
                    delegate_ballot.power -= reduction;
                    remove_vote(&delegate_ballot.vote, reduction)?;
                    ballots.save(deps.storage, (proposal_id, &delegate), &delegate_ballot)?;
                }
            }
        }
    }

    Ok(())
}
