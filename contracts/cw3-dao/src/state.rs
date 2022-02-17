use crate::msg::Threshold;
use cosmwasm_std::{
    Addr, BlockInfo, CosmosMsg, Decimal, Empty, StdError, StdResult, Storage, Uint128,
};
use cw3::{Status, Vote};
use cw_storage_plus::{Item, Map};
use cw_utils::{Duration, Expiration};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::convert::TryInto;

// we multiply by this when calculating needed_votes in order to round up properly
// Note: `10u128.pow(9)` fails as "u128::pow` is not yet stable as a const fn"
const PRECISION_FACTOR: u128 = 1_000_000_000;

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Config {
    pub name: String,
    pub description: String,
    pub threshold: Threshold,
    pub max_voting_period: Duration,
    pub proposal_deposit: Uint128,
    pub refund_failed_proposals: Option<bool>,
    pub image_url: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Proposal {
    pub title: String,
    pub description: String,
    pub proposer: Addr,
    pub start_height: u64,
    pub expires: Expiration,
    pub msgs: Vec<CosmosMsg<Empty>>,
    pub status: Status,
    /// Pass requirements
    pub threshold: Threshold,
    /// The total weight when the proposal started (used to calculate percentages)
    pub total_weight: Uint128,
    /// summary of existing votes
    pub votes: Votes,
    /// Amount of the native governance token required for voting
    pub deposit: Uint128,
}

// weight of votes for each option
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Votes {
    pub yes: Uint128,
    pub no: Uint128,
    pub abstain: Uint128,
    pub veto: Uint128,
}

impl Votes {
    /// sum of all votes
    pub fn total(&self) -> Uint128 {
        self.yes + self.no + self.abstain + self.veto
    }

    /// create it with a yes vote for this much
    pub fn new(init_weight: Uint128) -> Self {
        Votes {
            yes: init_weight,
            no: Uint128::zero(),
            abstain: Uint128::zero(),
            veto: Uint128::zero(),
        }
    }

    pub fn add_vote(&mut self, vote: Vote, weight: Uint128) {
        match vote {
            Vote::Yes => self.yes += weight,
            Vote::Abstain => self.abstain += weight,
            Vote::No => self.no += weight,
            Vote::Veto => self.veto += weight,
        }
    }
}

impl Proposal {
    /// current_status is non-mutable and returns what the status should be.
    /// (designed for queries)
    pub fn current_status(&self, block: &BlockInfo) -> Status {
        let mut status = self.status;

        // if open, check if voting is passed or timed out
        if status == Status::Open && self.is_passed(block) {
            status = Status::Passed;
        }
        if status == Status::Open && self.is_rejected(block) {
            status = Status::Rejected;
        }
        if status == Status::Open && self.expires.is_expired(block) {
            status = Status::Rejected;
        }

        status
    }

    /// update_status sets the status of the proposal to current_status.
    /// (designed for handler logic)
    pub fn update_status(&mut self, block: &BlockInfo) {
        self.status = self.current_status(block);
    }

    // returns true iff this proposal is sure to pass (even before expiration if no future
    // sequence of possible votes can cause it to fail)
    pub fn is_passed(&self, block: &BlockInfo) -> bool {
        match self.threshold {
            Threshold::AbsolutePercentage {
                percentage: percentage_needed,
            } => {
                self.votes.yes
                    >= votes_needed(self.total_weight - self.votes.abstain, percentage_needed)
            }
            Threshold::ThresholdQuorum { threshold, quorum } => {
                // we always require the quorum
                if self.votes.total() < votes_needed(self.total_weight, quorum) {
                    return false;
                }
                if self.expires.is_expired(block) {
                    // If expired, we compare Yes votes against the total number of votes (minus abstain).
                    let opinions = self.votes.total() - self.votes.abstain;
                    self.votes.yes >= votes_needed(opinions, threshold)
                } else {
                    // If not expired, we must assume all non-votes will be cast as No.
                    // We compare threshold against the total weight (minus abstain).
                    let possible_opinions = self.total_weight - self.votes.abstain;
                    self.votes.yes >= votes_needed(possible_opinions, threshold)
                }
            }
        }
    }

    /// As above for the rejected check, used to check if a proposal is
    /// already rejected.
    pub fn is_rejected(&self, block: &BlockInfo) -> bool {
        match self.threshold {
            Threshold::AbsolutePercentage {
                percentage: percentage_needed,
            } => {
                self.votes.no
                    >= votes_needed(self.total_weight - self.votes.abstain, percentage_needed)
            }
            Threshold::ThresholdQuorum { threshold, quorum } => {
                // we always require the quorum
                if self.votes.total() < votes_needed(self.total_weight, quorum) {
                    return false;
                }
                if self.expires.is_expired(block) {
                    // If expired, we compare No votes against the total number of votes (minus abstain).
                    let opinions = self.votes.total() - self.votes.abstain;
                    self.votes.no >= votes_needed(opinions, threshold)
                } else {
                    // If not expired, we must assume all non-votes will be cast as Yes.
                    // We compare threshold against the total weight (minus abstain).
                    let possible_opinions = self.total_weight - self.votes.abstain;
                    self.votes.no >= votes_needed(possible_opinions, threshold)
                }
            }
        }
    }
}

// this is a helper function so Decimal works with u64 rather than Uint128
// also, we must *round up* here, as we need 8, not 7 votes to reach 50% of 15 total
fn votes_needed(weight: Uint128, percentage: Decimal) -> Uint128 {
    let applied = percentage * Uint128::from(PRECISION_FACTOR * weight.u128());
    // Divide by PRECISION_FACTOR, rounding up to the nearest integer
    Uint128::from((applied.u128() + PRECISION_FACTOR - 1) / PRECISION_FACTOR)
}

// we cast a ballot with our chosen vote and a given weight
// stored under the key that voted
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Ballot {
    pub weight: Uint128,
    pub vote: Vote,
}

// Unique items
pub const CONFIG: Item<Config> = Item::new("config");
pub const PROPOSAL_COUNT: Item<u64> = Item::new("proposal_count");
pub const DAO_PAUSED: Item<Expiration> = Item::new("dao_paused");

// Total weight and voters are queried from this contract
pub const STAKING_CONTRACT: Item<Addr> = Item::new("staking_contract");

// Address of the token used for staking
pub const GOV_TOKEN: Item<Addr> = Item::new("gov_token");

// Stores staking contract CODE ID and Unbonding time for use in a reply
pub const STAKING_CONTRACT_CODE_ID: Item<u64> = Item::new("staking_contract_code_id");
pub const STAKING_CONTRACT_UNSTAKING_DURATION: Item<Option<Duration>> =
    Item::new("staking_contract_unstaking_duration");

// Multiple-item map
pub const BALLOTS: Map<(u64, &Addr), Ballot> = Map::new("votes");
pub const PROPOSALS: Map<u64, Proposal> = Map::new("proposals");
pub const TREASURY_TOKENS: Map<&Addr, Empty> = Map::new("treasury_tokens");

pub fn next_id(store: &mut dyn Storage) -> StdResult<u64> {
    let id: u64 = PROPOSAL_COUNT.may_load(store)?.unwrap_or_default() + 1;
    PROPOSAL_COUNT.save(store, &id)?;
    Ok(id)
}

pub fn parse_id(data: &[u8]) -> StdResult<u64> {
    match data[0..8].try_into() {
        Ok(bytes) => Ok(u64::from_be_bytes(bytes)),
        Err(_) => Err(StdError::generic_err(
            "Corrupted data found. 8 byte expected.",
        )),
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use cosmwasm_std::testing::mock_env;

    #[test]
    fn count_votes() {
        let mut votes = Votes::new(Uint128::new(5));
        votes.add_vote(Vote::No, Uint128::new(10));
        votes.add_vote(Vote::Veto, Uint128::new(20));
        votes.add_vote(Vote::Yes, Uint128::new(30));
        votes.add_vote(Vote::Abstain, Uint128::new(40));

        assert_eq!(votes.total(), Uint128::new(105));
        assert_eq!(votes.yes, Uint128::new(35));
        assert_eq!(votes.no, Uint128::new(10));
        assert_eq!(votes.veto, Uint128::new(20));
        assert_eq!(votes.abstain, Uint128::new(40));
    }

    #[test]
    // we ensure this rounds up (as it calculates needed votes)
    fn votes_needed_rounds_properly() {
        // round up right below 1
        assert_eq!(
            Uint128::new(1),
            votes_needed(Uint128::new(3), Decimal::permille(333))
        );
        // round up right over 1
        assert_eq!(
            Uint128::new(2),
            votes_needed(Uint128::new(3), Decimal::permille(334))
        );
        assert_eq!(
            Uint128::new(11),
            votes_needed(Uint128::new(30), Decimal::permille(334))
        );

        // exact matches don't round
        assert_eq!(
            Uint128::new(17),
            votes_needed(Uint128::new(34), Decimal::percent(50))
        );
        assert_eq!(
            Uint128::new(12),
            votes_needed(Uint128::new(48), Decimal::percent(25))
        );
    }

    fn setup_prop(
        threshold: Threshold,
        votes: Votes,
        total_weight: Uint128,
        is_expired: bool,
    ) -> (Proposal, BlockInfo) {
        let block = mock_env().block;
        let expires = match is_expired {
            true => Expiration::AtHeight(block.height - 5),
            false => Expiration::AtHeight(block.height + 100),
        };
        let prop = Proposal {
            title: "Demo".to_string(),
            description: "Info".to_string(),
            proposer: Addr::unchecked("test"),
            start_height: 100,
            expires,
            msgs: vec![],
            status: Status::Open,
            threshold,
            total_weight,
            votes,
            deposit: Uint128::zero(),
        };
        (prop, block)
    }

    fn check_is_passed(
        threshold: Threshold,
        votes: Votes,
        total_weight: Uint128,
        is_expired: bool,
    ) -> bool {
        let (prop, block) = setup_prop(threshold, votes, total_weight, is_expired);
        prop.is_passed(&block)
    }

    fn check_is_rejected(
        threshold: Threshold,
        votes: Votes,
        total_weight: Uint128,
        is_expired: bool,
    ) -> bool {
        let (prop, block) = setup_prop(threshold, votes, total_weight, is_expired);
        prop.is_rejected(&block)
    }

    #[test]
    fn proposal_passed_absolute_percentage() {
        let percent = Threshold::AbsolutePercentage {
            percentage: Decimal::percent(50),
        };
        let mut votes = Votes::new(Uint128::new(7));
        votes.add_vote(Vote::No, Uint128::new(4));
        votes.add_vote(Vote::Abstain, Uint128::new(2));
        // same expired or not, if yes >= ceiling(0.5 * (total - abstained))
        // 7 of (15-2) passes
        assert!(check_is_passed(
            percent.clone(),
            votes.clone(),
            Uint128::new(15),
            false
        ));
        assert!(check_is_passed(
            percent.clone(),
            votes.clone(),
            Uint128::new(15),
            true
        ));
        // but 7 of (17-2) fails
        assert!(!check_is_passed(
            percent.clone(),
            votes.clone(),
            Uint128::new(17),
            false
        ));

        // if the total were a bit lower, this would pass
        assert!(check_is_passed(
            percent.clone(),
            votes.clone(),
            Uint128::new(14),
            false
        ));
        assert!(check_is_passed(percent, votes, Uint128::new(14), true));
    }

    #[test]
    fn proposal_rejected_absolute_percentage() {
        let percent = Threshold::AbsolutePercentage {
            percentage: Decimal::percent(50),
        };

        // 4 YES, 7 NO, 2 ABSTAIN
        let mut votes = Votes::new(Uint128::new(4));
        votes.add_vote(Vote::No, Uint128::new(7));
        votes.add_vote(Vote::Abstain, Uint128::new(2));

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
        assert!(check_is_rejected(
            percent,
            votes.clone(),
            Uint128::new(14),
            true
        ));
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
            veto: Uint128::new(1),
        };
        // abstain votes are not counted for threshold => yes / (yes + no + veto)
        let passes_ignoring_abstain = Votes {
            yes: Uint128::new(6),
            no: Uint128::new(4),
            abstain: Uint128::new(5),
            veto: Uint128::new(2),
        };
        // fails any way you look at it
        let failing = Votes {
            yes: Uint128::new(6),
            no: Uint128::new(5),
            abstain: Uint128::new(2),
            veto: Uint128::new(2),
        };

        // first, expired (voting period over)
        // over quorum (40% of 30 = 12), over threshold (7/11 > 50%)
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
            no: Uint128::new(7),
            abstain: Uint128::new(2),
            veto: Uint128::new(1),
        };
        // abstain votes are not counted for threshold => yes / (yes + no + veto)
        let rejected_ignoring_abstain = Votes {
            yes: Uint128::new(4),
            no: Uint128::new(6),
            abstain: Uint128::new(5),
            veto: Uint128::new(2),
        };
        // fails any way you look at it
        let failing = Votes {
            yes: Uint128::new(5),
            no: Uint128::new(6),
            abstain: Uint128::new(2),
            veto: Uint128::new(2),
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

        // over quorum, but under threshold fails also
        assert!(!check_is_rejected(
            quorum.clone(),
            failing,
            Uint128::new(20),
            true
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
        // when we pass absolute threshold (everyone else voting no, we pass), but still don't hit quorum
        let quorum = Threshold::ThresholdQuorum {
            threshold: Decimal::percent(60),
            quorum: Decimal::percent(80),
        };

        // try 9 yes, 1 no (out of 15) -> 90% voter threshold, 60% absolute threshold, still no quorum
        // doesn't matter if expired or not
        let missing_voters = Votes {
            yes: Uint128::new(9),
            no: Uint128::new(1),
            abstain: Uint128::new(0),
            veto: Uint128::new(0),
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

        // 1 less yes, 3 vetos and this passes only when expired
        let wait_til_expired = Votes {
            yes: Uint128::new(8),
            no: Uint128::new(1),
            abstain: Uint128::new(0),
            veto: Uint128::new(3),
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
            veto: Uint128::new(0),
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
