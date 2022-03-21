use cosmwasm_std::{Addr, BlockInfo, CosmosMsg, Empty, StdResult, Storage, Uint128};
use cw_utils::Expiration;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use voting::{votes_meet_threshold, Votes};

use crate::{
    query::ProposalResponse,
    state::{CheckedDepositInfo, PROPOSAL_COUNT},
    threshold::Threshold,
};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug, Copy)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum Status {
    /// The proposal is open for voting.
    Open,
    /// The proposal has been rejected.
    Rejected,
    /// The proposal has been passed but has not been executed.
    Passed,
    /// The proposal has been passed and executed.
    Executed,
    /// The proposal has failed or expired and has been closed. A
    /// proposal deposit refund has been issued if applicable.
    Closed,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Proposal {
    pub title: String,
    pub description: String,
    pub proposer: Addr,

    pub start_height: u64,
    pub expiration: Expiration,

    pub threshold: Threshold,
    pub total_power: Uint128,

    pub msgs: Vec<CosmosMsg<Empty>>,

    pub status: Status,
    pub votes: Votes,

    /// Information about the deposit that was sent as part of this
    /// proposal. None if no deposit.
    pub deposit_info: Option<CheckedDepositInfo>,
}

pub fn advance_proposal_id(store: &mut dyn Storage) -> StdResult<u64> {
    let id: u64 = PROPOSAL_COUNT.may_load(store)?.unwrap_or_default() + 1;
    PROPOSAL_COUNT.save(store, &id)?;
    Ok(id)
}

impl Proposal {
    /// Consumes the proposal and returns a version which may be used
    /// in a query response. The difference being that proposal
    /// statuses are only updated on vote, execute, and close
    /// events. It is possible though that since a vote has occured
    /// the proposal expiring has changed its status. This method
    /// recomputes the status so that queries get accurate
    /// information.
    pub fn into_response(mut self, block: &BlockInfo, id: u64) -> ProposalResponse {
        self.update_status(block);
        ProposalResponse { id, proposal: self }
    }

    /// Gets the current status of the proposal.
    pub fn current_status(&self, block: &BlockInfo) -> Status {
        if self.status == Status::Open && self.is_passed(block) {
            Status::Passed
        } else if self.status == Status::Open
            && (self.expiration.is_expired(block) || self.is_rejected(block))
        {
            Status::Rejected
        } else {
            self.status
        }
    }

    /// Sets a proposals status to its current status.
    pub fn update_status(&mut self, block: &BlockInfo) {
        self.status = self.current_status(block)
    }

    /// Returns true iff this proposal is sure to pass (even before
    /// expiration if no future sequence of possible votes can cause
    /// it to fail)
    pub fn is_passed(&self, block: &BlockInfo) -> bool {
        self.does_vote_count_reach_threshold(self.votes.yes, block)
    }

    /// As above for the passed check, used to check if a proposal is
    /// already rejected.
    pub fn is_rejected(&self, block: &BlockInfo) -> bool {
        self.does_vote_count_reach_threshold(self.votes.no, block)
    }

    /// Helper function to determine if a vote count has reached the
    /// proposal's threshold. Useful for determining if a particular
    /// outcome has been reached, for example
    /// `does_vote_count_meet_threshold(self.votes.yes, block)` will
    /// return true if there are a sufficent number of yes votes to
    /// pass the proposal.
    fn does_vote_count_reach_threshold(&self, vote_count: Uint128, block: &BlockInfo) -> bool {
        match self.threshold {
            Threshold::AbsolutePercentage { percentage } => votes_meet_threshold(
                vote_count,
                self.total_power - self.votes.abstain,
                percentage,
            ),
            Threshold::ThresholdQuorum { threshold, quorum } => {
                if !votes_meet_threshold(self.votes.total(), self.total_power, quorum) {
                    return false;
                }
                if self.expiration.is_expired(block) {
                    // If the quorum is met and the proposal is
                    // expired the number of votes needed to pass a
                    // proposal is compared to the number of votes on
                    // the proposal.
                    //
                    // NOTE(zeke): I do not like this
                    // behavior. Strongly recomend that we either
                    // remove custom end dates from proposals or
                    // remove this logic. These together make the cost
                    // of doing a hostile takeover of the DAO
                    // `token_price * quorum` as opposed to the
                    // 'desired' `token_price * threshold`.
                    let options = self.votes.total() - self.votes.abstain;
                    votes_meet_threshold(vote_count, options, threshold)
                } else {
                    let options = self.total_power - self.votes.abstain;
                    votes_meet_threshold(vote_count, options, threshold)
                }
            }
        }
    }
}

impl std::fmt::Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Status::Open => write!(f, "open"),
            Status::Rejected => write!(f, "rejected"),
            Status::Passed => write!(f, "passed"),
            Status::Executed => write!(f, "executed"),
            Status::Closed => write!(f, "closed"),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use cosmwasm_std::{testing::mock_env, Decimal};

    fn setup_prop(
        threshold: Threshold,
        votes: Votes,
        total_power: Uint128,
        is_expired: bool,
    ) -> (Proposal, BlockInfo) {
        let block = mock_env().block;
        let expiration = match is_expired {
            true => Expiration::AtHeight(block.height - 5),
            false => Expiration::AtHeight(block.height + 100),
        };
        let prop = Proposal {
            title: "Demo".to_string(),
            description: "Info".to_string(),
            proposer: Addr::unchecked("test"),
            start_height: 100,
            expiration,
            msgs: vec![],
            status: Status::Open,
            threshold,
            total_power,
            votes,
            deposit_info: None,
        };
        (prop, block)
    }

    fn check_is_passed(
        threshold: Threshold,
        votes: Votes,
        total_power: Uint128,
        is_expired: bool,
    ) -> bool {
        let (prop, block) = setup_prop(threshold, votes, total_power, is_expired);
        prop.is_passed(&block)
    }

    fn check_is_rejected(
        threshold: Threshold,
        votes: Votes,
        total_power: Uint128,
        is_expired: bool,
    ) -> bool {
        let (prop, block) = setup_prop(threshold, votes, total_power, is_expired);
        prop.is_rejected(&block)
    }

    #[test]
    fn test_pass_absolute_percentage() {
        let threshold = Threshold::AbsolutePercentage {
            percentage: Decimal::percent(50),
        };
        let votes = Votes {
            yes: Uint128::new(7),
            no: Uint128::new(4),
            abstain: Uint128::new(2),
        };

        // 15 total votes. 7 yes and 2 abstain. 50% threshold. Should
        // be passed.
        assert!(check_is_passed(
            threshold.clone(),
            votes.clone(),
            Uint128::new(15),
            false
        ));
        // Proposal being expired should not effect those results.
        assert!(check_is_passed(
            threshold.clone(),
            votes.clone(),
            Uint128::new(15),
            true
        ));

        // More votes == higher threshold => not passed.
        assert!(!check_is_passed(threshold, votes, Uint128::new(17), false));
    }

    #[test]
    fn test_reject_absolute_percentage() {
        let percent = Threshold::AbsolutePercentage {
            percentage: Decimal::percent(50),
        };

        // 4 YES, 7 NO, 2 ABSTAIN
        let votes = Votes {
            yes: Uint128::new(4),
            no: Uint128::new(7),
            abstain: Uint128::new(2),
        };

        // 15 total voting power
        // 7 / (15 - 2) > 50%
        // Expiry does not matter
        assert!(check_is_rejected(
            percent.clone(),
            votes.clone(),
            Uint128::new(15),
            false
        ));
        assert!(check_is_rejected(
            percent.clone(),
            votes.clone(),
            Uint128::new(15),
            true
        ));

        // 17 total voting power
        // 7 / (17 - 2) < 50%
        assert!(!check_is_rejected(
            percent.clone(),
            votes.clone(),
            Uint128::new(17),
            false
        ));
        assert!(!check_is_rejected(
            percent.clone(),
            votes.clone(),
            Uint128::new(17),
            true
        ));

        // Rejected if total was lower
        assert!(check_is_rejected(
            percent.clone(),
            votes.clone(),
            Uint128::new(14),
            false
        ));
        assert!(check_is_rejected(percent, votes, Uint128::new(14), true));
    }

    #[test]
    fn proposal_passed_quorum() {
        let quorum = Threshold::ThresholdQuorum {
            threshold: Decimal::percent(50),
            quorum: Decimal::percent(40),
        };
        // all non-yes votes are counted for quorum
        let passing = Votes {
            yes: Uint128::new(7),
            no: Uint128::new(3),
            abstain: Uint128::new(2),
        };
        // abstain votes are not counted for threshold => yes / (yes + no + veto)
        let passes_ignoring_abstain = Votes {
            yes: Uint128::new(6),
            no: Uint128::new(6),
            abstain: Uint128::new(5),
        };
        // fails any way you look at it
        let failing = Votes {
            yes: Uint128::new(6),
            no: Uint128::new(7),
            abstain: Uint128::new(2),
        };

        // first, expired (voting period over)
        // over quorum (40% of 30 = 12), over threshold (7/12 > 50%)
        assert!(check_is_passed(
            quorum.clone(),
            passing.clone(),
            Uint128::new(30),
            true
        ));
        // under quorum it is not passing (40% of 33 = 13.2 > 13)
        assert!(!check_is_passed(
            quorum.clone(),
            passing.clone(),
            Uint128::new(33),
            true
        ));
        // over quorum, threshold passes if we ignore abstain
        // 17 total votes w/ abstain => 40% quorum of 40 total
        // 6 yes / (6 yes + 4 no + 2 votes) => 50% threshold
        assert!(check_is_passed(
            quorum.clone(),
            passes_ignoring_abstain.clone(),
            Uint128::new(40),
            true
        ));
        // over quorum, but under threshold fails also
        assert!(!check_is_passed(
            quorum.clone(),
            failing,
            Uint128::new(20),
            true
        ));

        // now, check with open voting period
        // would pass if closed, but fail here, as remaining votes no -> fail
        assert!(!check_is_passed(
            quorum.clone(),
            passing.clone(),
            Uint128::new(30),
            false
        ));
        assert!(!check_is_passed(
            quorum.clone(),
            passes_ignoring_abstain.clone(),
            Uint128::new(40),
            false
        ));
        // if we have threshold * total_weight as yes votes this must pass
        assert!(check_is_passed(
            quorum.clone(),
            passing.clone(),
            Uint128::new(14),
            false
        ));
        // all votes have been cast, some abstain
        assert!(check_is_passed(
            quorum.clone(),
            passes_ignoring_abstain,
            Uint128::new(17),
            false
        ));
        // 3 votes uncast, if they all vote no, we have 7 yes, 7 no+veto, 2 abstain (out of 16)
        assert!(check_is_passed(quorum, passing, Uint128::new(16), false));
    }

    #[test]
    fn proposal_rejected_quorum() {
        let quorum = Threshold::ThresholdQuorum {
            threshold: Decimal::percent(50),
            quorum: Decimal::percent(40),
        };
        // all non-yes votes are counted for quorum
        let rejecting = Votes {
            yes: Uint128::new(3),
            no: Uint128::new(8),
            abstain: Uint128::new(2),
        };
        // abstain votes are not counted for threshold => yes / (yes + no)
        let rejected_ignoring_abstain = Votes {
            yes: Uint128::new(4),
            no: Uint128::new(8),
            abstain: Uint128::new(5),
        };
        // fails any way you look at it
        let failing = Votes {
            yes: Uint128::new(5),
            no: Uint128::new(8),
            abstain: Uint128::new(2),
        };

        // first, expired (voting period over)
        // over quorum (40% of 30 = 12), over threshold (7/11 > 50%)
        assert!(check_is_rejected(
            quorum.clone(),
            rejecting.clone(),
            Uint128::new(30),
            true
        ));
        // Under quorum means it cannot be rejected
        assert!(!check_is_rejected(
            quorum.clone(),
            rejecting.clone(),
            Uint128::new(33),
            true
        ));

        // over quorum, threshold passes if we ignore abstain
        // 17 total votes w/ abstain => 40% quorum of 40 total
        // 6 no / (6 no + 4 yes + 2 votes) => 50% threshold
        assert!(check_is_rejected(
            quorum.clone(),
            rejected_ignoring_abstain.clone(),
            Uint128::new(40),
            true
        ));

        // Over quorum, but under threshold fails if the proposal is
        // not expired. If the proposal is expired though it passes as
        // the total vote count used is the number of votes, and not
        // the total number of votes avaliable.
        assert!(check_is_rejected(
            quorum.clone(),
            failing.clone(),
            Uint128::new(20),
            true
        ));
        assert!(!check_is_rejected(
            quorum.clone(),
            failing,
            Uint128::new(20),
            false
        ));

        // Voting is still open so assume rest of votes are yes
        // threshold not reached
        assert!(!check_is_rejected(
            quorum.clone(),
            rejecting.clone(),
            Uint128::new(30),
            false
        ));
        assert!(!check_is_rejected(
            quorum.clone(),
            rejected_ignoring_abstain.clone(),
            Uint128::new(40),
            false
        ));
        // if we have threshold * total_weight as no votes this must reject
        assert!(check_is_rejected(
            quorum.clone(),
            rejecting.clone(),
            Uint128::new(14),
            false
        ));
        // all votes have been cast, some abstain
        assert!(check_is_rejected(
            quorum.clone(),
            rejected_ignoring_abstain,
            Uint128::new(17),
            false
        ));
        // 3 votes uncast, if they all vote yes, we have 7 no, 7 yes+veto, 2 abstain (out of 16)
        assert!(check_is_rejected(
            quorum,
            rejecting,
            Uint128::new(16),
            false
        ));
    }

    #[test]
    fn quorum_edge_cases() {
        // When we pass absolute threshold (everyone else voting no,
        // we pass), but still don't hit quorum.
        let quorum = Threshold::ThresholdQuorum {
            threshold: Decimal::percent(60),
            quorum: Decimal::percent(80),
        };

        // Try 9 yes, 1 no (out of 15) -> 90% voter threshold, 60%
        // absolute threshold, still no quorum doesn't matter if
        // expired or not.
        let missing_voters = Votes {
            yes: Uint128::new(9),
            no: Uint128::new(1),
            abstain: Uint128::new(0),
        };
        assert!(!check_is_passed(
            quorum.clone(),
            missing_voters.clone(),
            Uint128::new(15),
            false
        ));
        assert!(!check_is_passed(
            quorum.clone(),
            missing_voters,
            Uint128::new(15),
            true
        ));

        // 1 less yes, 3 vetos and this passes only when expired.
        let wait_til_expired = Votes {
            yes: Uint128::new(8),
            no: Uint128::new(4),
            abstain: Uint128::new(0),
        };
        assert!(!check_is_passed(
            quorum.clone(),
            wait_til_expired.clone(),
            Uint128::new(15),
            false
        ));
        assert!(check_is_passed(
            quorum.clone(),
            wait_til_expired,
            Uint128::new(15),
            true
        ));

        // 9 yes and 3 nos passes early
        let passes_early = Votes {
            yes: Uint128::new(9),
            no: Uint128::new(3),
            abstain: Uint128::new(0),
        };
        assert!(check_is_passed(
            quorum.clone(),
            passes_early.clone(),
            Uint128::new(15),
            false
        ));
        assert!(check_is_passed(
            quorum,
            passes_early,
            Uint128::new(15),
            true
        ));
    }
}
