use cosmwasm_std::{Addr, BlockInfo, StdError, StdResult, Uint128};
use cw_utils::Expiration;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use voting::{
    deposit::CheckedDepositInfo,
    proposal::Proposal,
    status::Status,
    voting::{does_vote_count_pass, MultipleChoiceVotes},
};

use crate::{
    query::ProposalResponse,
    state::{CheckedMultipleChoiceOption, MultipleChoiceOptionType},
    voting_strategy::VotingStrategy,
};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct MultipleChoiceProposal {
    pub title: String,
    pub description: String,
    pub proposer: Addr,
    pub start_height: u64,
    /// The minimum amount of time this proposal must remain open for
    /// voting. The proposal may not pass unless this is expired or
    /// None.
    pub min_voting_period: Option<Expiration>,
    pub expiration: Expiration,
    pub choices: Vec<CheckedMultipleChoiceOption>,
    pub status: Status,
    pub voting_strategy: VotingStrategy,
    /// The total power when the proposal started (used to calculate percentages)
    pub total_power: Uint128,
    pub votes: MultipleChoiceVotes,
    /// Information about the deposit that was sent as part of this
    /// proposal. None if no deposit.
    pub deposit_info: Option<CheckedDepositInfo>,
}

pub enum VoteResult {
    SingleWinner(CheckedMultipleChoiceOption),
    Tie,
}

impl Proposal for MultipleChoiceProposal {
    fn proposer(&self) -> Addr {
        self.proposer.clone()
    }
    fn deposit_info(&self) -> Option<CheckedDepositInfo> {
        self.deposit_info.clone()
    }
    fn status(&self) -> Status {
        self.status
    }
}

impl MultipleChoiceProposal {
    /// Consumes the proposal and returns a version which may be used
    /// in a query response. The difference being that proposal
    /// statuses are only updated on vote, execute, and close
    /// events. It is possible though that since a vote has occured
    /// the proposal expiring has changed its status. This method
    /// recomputes the status so that queries get accurate
    /// information.
    pub fn into_response(mut self, block: &BlockInfo, id: u64) -> StdResult<ProposalResponse> {
        self.update_status(block)?;
        Ok(ProposalResponse { id, proposal: self })
    }

    /// Gets the current status of the proposal.
    pub fn current_status(&self, block: &BlockInfo) -> StdResult<Status> {
        if self.status == Status::Open && self.is_passed(block)? {
            Ok(Status::Passed)
        } else if self.status == Status::Open
            && (self.expiration.is_expired(block) || self.is_rejected(block)?)
        {
            Ok(Status::Rejected)
        } else {
            Ok(self.status)
        }
    }

    /// Sets a proposals status to its current status.
    pub fn update_status(&mut self, block: &BlockInfo) -> StdResult<()> {
        self.status = self.current_status(block)?;
        Ok(())
    }

    /// Returns true iff this proposal is sure to pass (even before
    /// expiration if no future sequence of possible votes can cause
    /// it to fail). Passing in the case of multiple choice proposals
    /// means that quorum has been met,
    /// one of the options that is not "None of the above"
    /// has won the most votes, and there is no tie.
    pub fn is_passed(&self, block: &BlockInfo) -> StdResult<bool> {
        // If the min voting period is set and not expired the
        // proposal can not yet be passed. This gives DAO members some
        // time to remove liquidity / scheme on a recovery plan if a
        // single actor accumulates enough tokens to unilaterally pass
        // proposals.
        if let Some(min) = self.min_voting_period {
            if !min.is_expired(block) {
                return Ok(false);
            }
        }

        // Proposal can only pass if quorum has been met.
        if does_vote_count_pass(
            self.votes.total(),
            self.total_power,
            self.voting_strategy.get_quorum(),
        ) {
            let vote_result = self.calculate_vote_result()?;
            match vote_result {
                // Proposal is not passed if there is a tie.
                VoteResult::Tie => return Ok(false),
                VoteResult::SingleWinner(winning_choice) => {
                    // Proposal is not passed if winning choice is None.
                    if winning_choice.option_type != MultipleChoiceOptionType::None {
                        // If proposal is expired, quorum has been reached, and winning choice is neither tied nor None, then proposal is passed.
                        if self.expiration.is_expired(block) {
                            return Ok(true);
                        } else {
                            // If the proposal is not expired but the leading choice cannot
                            // possibly be outwon by any other choices, the proposal has passed.
                            return self.is_choice_unbeatable(&winning_choice);
                        }
                    }
                }
            }
        }
        Ok(false)
    }

    pub fn is_rejected(&self, block: &BlockInfo) -> StdResult<bool> {
        let vote_result = self.calculate_vote_result()?;
        match vote_result {
            // Proposal is rejected if there is a tie, and either the proposal is expired or
            // there is no voting power left.
            VoteResult::Tie => {
                let rejected =
                    self.expiration.is_expired(block) || self.total_power == self.votes.total();
                Ok(rejected)
            }
            VoteResult::SingleWinner(winning_choice) => {
                match (
                    does_vote_count_pass(
                        self.votes.total(),
                        self.total_power,
                        self.voting_strategy.get_quorum(),
                    ),
                    self.expiration.is_expired(block),
                ) {
                    // Quorum is met and proposal is expired.
                    (true, true) => {
                        // Proposal is rejected if "None" is the winning option.
                        if winning_choice.option_type == MultipleChoiceOptionType::None {
                            return Ok(true);
                        }
                        Ok(false)
                    }
                    // Proposal is not expired, quorum is either is met or unmet.
                    (true, false) | (false, false) => {
                        // If the proposal is not expired and the leading choice is None and it cannot
                        // possibly be outwon by any other choices, the proposal is rejected.
                        if winning_choice.option_type == MultipleChoiceOptionType::None {
                            return self.is_choice_unbeatable(&winning_choice);
                        }
                        Ok(false)
                    }
                    // Quorum is not met and proposal is expired.
                    (false, true) => Ok(true),
                }
            }
        }
    }

    /// Find the option with the highest vote weight, and note if there is a tie.
    pub fn calculate_vote_result(&self) -> StdResult<VoteResult> {
        match self.voting_strategy {
            VotingStrategy::SingleChoice { quorum: _ } => {
                // We expect to have at least 3 vote weights
                if let Some(max_weight) = self.votes.vote_weights.iter().max_by(|&a, &b| a.cmp(b)) {
                    let top_choices: Vec<(usize, &Uint128)> = self
                        .votes
                        .vote_weights
                        .iter()
                        .enumerate()
                        .filter(|x| x.1 == max_weight)
                        .collect();

                    // If more than one choice has the highest number of votes, we have a tie.
                    if top_choices.len() > 1 {
                        return Ok(VoteResult::Tie);
                    }

                    match top_choices.first() {
                        Some(winning_choice) => {
                            return Ok(VoteResult::SingleWinner(
                                self.choices[winning_choice.0].clone(),
                            ));
                        }
                        None => {
                            return Err(StdError::generic_err("no votes found"));
                        }
                    }
                }
                Err(StdError::not_found("max vote weight"))
            }
        }
    }

    /// Ensure that with the remaining vote power, the choice with the second highest votes
    /// cannot overtake the first choice.
    fn is_choice_unbeatable(
        &self,
        winning_choice: &CheckedMultipleChoiceOption,
    ) -> StdResult<bool> {
        let winning_choice_power = self.votes.vote_weights[winning_choice.index as usize];
        if let Some(second_choice_power) = self
            .votes
            .vote_weights
            .iter()
            .filter(|&x| x < &winning_choice_power)
            .max_by(|&a, &b| a.cmp(b))
        {
            // Check if the remaining vote power can be used to overtake the current winning choice.
            let remaining_vote_power = self.total_power - self.votes.total();
            match winning_choice.option_type {
                MultipleChoiceOptionType::Standard => {
                    if winning_choice_power > *second_choice_power + remaining_vote_power {
                        return Ok(true);
                    }
                }
                MultipleChoiceOptionType::None => {
                    // If the winning choice is None, and we can at most achieve a tie,
                    // this choice is unbeatable because a tie will also fail the proposal. This is why we check for '>=' in this case
                    // rather than '>'.
                    if winning_choice_power >= *second_choice_power + remaining_vote_power {
                        return Ok(true);
                    }
                }
            }
        } else {
            return Err(StdError::not_found("second highest vote weight"));
        }
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use crate::state::{MultipleChoiceOption, MultipleChoiceOptions};

    use super::*;

    use cosmwasm_std::testing::mock_env;

    fn create_proposal(
        block: &BlockInfo,
        voting_strategy: VotingStrategy,
        votes: MultipleChoiceVotes,
        total_power: Uint128,
        is_expired: bool,
    ) -> MultipleChoiceProposal {
        // The last option that gets added in into_checked is always the none of the above option
        let options = vec![
            MultipleChoiceOption {
                description: "multiple choice option 1".to_string(),
                msgs: None,
            },
            MultipleChoiceOption {
                description: "multiple choice option 2".to_string(),
                msgs: None,
            },
        ];

        let expiration: Expiration = if is_expired {
            Expiration::AtHeight(block.height - 5)
        } else {
            Expiration::AtHeight(block.height + 5)
        };

        let mc_options = MultipleChoiceOptions { options };
        MultipleChoiceProposal {
            title: "A simple text proposal".to_string(),
            description: "A simple text proposal".to_string(),
            proposer: Addr::unchecked("CREATOR"),
            start_height: mock_env().block.height,
            expiration,
            // The last option that gets added in into_checked is always the none of the above option
            choices: mc_options.into_checked().unwrap().options,
            status: Status::Open,
            voting_strategy,
            total_power,
            votes,
            deposit_info: None,
            min_voting_period: None,
        }
    }

    #[test]
    fn test_majority_quorum() {
        let env = mock_env();
        let voting_strategy = VotingStrategy::SingleChoice {
            quorum: voting::threshold::PercentageThreshold::Majority {},
        };

        let votes = MultipleChoiceVotes {
            vote_weights: vec![Uint128::new(1), Uint128::new(0), Uint128::new(0)],
        };

        let prop = create_proposal(
            &env.block,
            voting_strategy.clone(),
            votes,
            Uint128::new(1),
            false,
        );

        // Quorum was met and all votes were cast, should be passed.
        assert!(prop.is_passed(&env.block).unwrap());
        assert!(!prop.is_rejected(&env.block).unwrap());

        let votes = MultipleChoiceVotes {
            vote_weights: vec![Uint128::new(0), Uint128::new(0), Uint128::new(1)],
        };
        let prop = create_proposal(
            &env.block,
            voting_strategy.clone(),
            votes,
            Uint128::new(1),
            false,
        );

        // Quorum was met but none of the above won, should be rejected.
        assert!(!prop.is_passed(&env.block).unwrap());
        assert!(prop.is_rejected(&env.block).unwrap());

        let votes = MultipleChoiceVotes {
            vote_weights: vec![Uint128::new(1), Uint128::new(0), Uint128::new(0)],
        };
        let prop = create_proposal(
            &env.block,
            voting_strategy.clone(),
            votes,
            Uint128::new(100),
            false,
        );

        // Quorum was not met and is not expired, should be open.
        assert!(!prop.is_passed(&env.block).unwrap());
        assert!(!prop.is_rejected(&env.block).unwrap());

        let votes = MultipleChoiceVotes {
            vote_weights: vec![Uint128::new(1), Uint128::new(0), Uint128::new(0)],
        };
        let prop = create_proposal(
            &env.block,
            voting_strategy.clone(),
            votes,
            Uint128::new(100),
            true,
        );

        // Quorum was not met and it is expired, should be rejected.
        assert!(!prop.is_passed(&env.block).unwrap());
        assert!(prop.is_rejected(&env.block).unwrap());

        let votes = MultipleChoiceVotes {
            vote_weights: vec![Uint128::new(50), Uint128::new(50), Uint128::new(0)],
        };
        let prop = create_proposal(
            &env.block,
            voting_strategy.clone(),
            votes,
            Uint128::new(100),
            true,
        );

        // Quorum was met but it is a tie and expired, should be rejected.
        assert!(!prop.is_passed(&env.block).unwrap());
        assert!(prop.is_rejected(&env.block).unwrap());

        let votes = MultipleChoiceVotes {
            vote_weights: vec![Uint128::new(50), Uint128::new(50), Uint128::new(0)],
        };
        let prop = create_proposal(&env.block, voting_strategy, votes, Uint128::new(150), false);

        // Quorum was met but it is a tie but not expired and still voting power remains, should be open.
        assert!(!prop.is_passed(&env.block).unwrap());
        assert!(!prop.is_rejected(&env.block).unwrap());
    }

    #[test]
    fn test_percentage_quorum() {
        let env = mock_env();
        let voting_strategy = VotingStrategy::SingleChoice {
            quorum: voting::threshold::PercentageThreshold::Percent(
                cosmwasm_std::Decimal::percent(10),
            ),
        };

        let votes = MultipleChoiceVotes {
            vote_weights: vec![Uint128::new(1), Uint128::new(0), Uint128::new(0)],
        };

        let prop = create_proposal(
            &env.block,
            voting_strategy.clone(),
            votes,
            Uint128::new(1),
            false,
        );

        // Quorum was met and all votes were cast, should be passed.
        assert!(prop.is_passed(&env.block).unwrap());
        assert!(!prop.is_rejected(&env.block).unwrap());

        let votes = MultipleChoiceVotes {
            vote_weights: vec![Uint128::new(0), Uint128::new(0), Uint128::new(1)],
        };
        let prop = create_proposal(
            &env.block,
            voting_strategy.clone(),
            votes,
            Uint128::new(1),
            false,
        );

        // Quorum was met but none of the above won, should be rejected.
        assert!(!prop.is_passed(&env.block).unwrap());
        assert!(prop.is_rejected(&env.block).unwrap());

        let votes = MultipleChoiceVotes {
            vote_weights: vec![Uint128::new(1), Uint128::new(0), Uint128::new(0)],
        };
        let prop = create_proposal(
            &env.block,
            voting_strategy.clone(),
            votes,
            Uint128::new(100),
            false,
        );

        // Quorum was not met and is not expired, should be open.
        assert!(!prop.is_passed(&env.block).unwrap());
        assert!(!prop.is_rejected(&env.block).unwrap());

        let votes = MultipleChoiceVotes {
            vote_weights: vec![Uint128::new(1), Uint128::new(0), Uint128::new(0)],
        };
        let prop = create_proposal(
            &env.block,
            voting_strategy.clone(),
            votes,
            Uint128::new(101),
            true,
        );

        // Quorum was not met and it is expired, should be rejected.
        assert!(!prop.is_passed(&env.block).unwrap());
        assert!(prop.is_rejected(&env.block).unwrap());

        let votes = MultipleChoiceVotes {
            vote_weights: vec![Uint128::new(50), Uint128::new(50), Uint128::new(0)],
        };
        let prop = create_proposal(
            &env.block,
            voting_strategy.clone(),
            votes,
            Uint128::new(10000),
            true,
        );

        // Quorum was met but it is a tie and expired, should be rejected.
        assert!(!prop.is_passed(&env.block).unwrap());
        assert!(prop.is_rejected(&env.block).unwrap());

        let votes = MultipleChoiceVotes {
            vote_weights: vec![Uint128::new(50), Uint128::new(50), Uint128::new(0)],
        };
        let prop = create_proposal(&env.block, voting_strategy, votes, Uint128::new(150), false);

        // Quorum was met but it is a tie but not expired and still voting power remains, should be open.
        assert!(!prop.is_passed(&env.block).unwrap());
        assert!(!prop.is_rejected(&env.block).unwrap());
    }

    #[test]
    fn test_unbeatable_none_option() {
        let env = mock_env();
        let voting_strategy = VotingStrategy::SingleChoice {
            quorum: voting::threshold::PercentageThreshold::Percent(
                cosmwasm_std::Decimal::percent(10),
            ),
        };
        let votes = MultipleChoiceVotes {
            vote_weights: vec![Uint128::new(0), Uint128::new(50), Uint128::new(500)],
        };
        let prop = create_proposal(
            &env.block,
            voting_strategy,
            votes,
            Uint128::new(1000),
            false,
        );

        // Quorum was met but none of the above is winning, but it also can't be beat (only a tie at best), should be rejected
        assert!(!prop.is_passed(&env.block).unwrap());
        assert!(prop.is_rejected(&env.block).unwrap());
    }

    #[test]
    fn test_quorum_rounding() {
        let env = mock_env();
        let voting_strategy = VotingStrategy::SingleChoice {
            quorum: voting::threshold::PercentageThreshold::Percent(
                cosmwasm_std::Decimal::percent(10),
            ),
        };
        let votes = MultipleChoiceVotes {
            vote_weights: vec![Uint128::new(10), Uint128::new(0), Uint128::new(0)],
        };
        let prop = create_proposal(&env.block, voting_strategy, votes, Uint128::new(100), true);

        // Quorum was met and proposal expired, should pass
        assert!(prop.is_passed(&env.block).unwrap());
        assert!(!prop.is_rejected(&env.block).unwrap());

        // High Precision rounding
        let voting_strategy = VotingStrategy::SingleChoice {
            quorum: voting::threshold::PercentageThreshold::Percent(
                cosmwasm_std::Decimal::percent(100),
            ),
        };

        let votes = MultipleChoiceVotes {
            vote_weights: vec![Uint128::new(999999), Uint128::new(0), Uint128::new(0)],
        };
        let prop = create_proposal(
            &env.block,
            voting_strategy,
            votes,
            Uint128::new(1000000),
            true,
        );

        // Quorum was not met and expired, should reject
        assert!(!prop.is_passed(&env.block).unwrap());
        assert!(prop.is_rejected(&env.block).unwrap());

        // High Precision rounding
        let voting_strategy = VotingStrategy::SingleChoice {
            quorum: voting::threshold::PercentageThreshold::Percent(
                cosmwasm_std::Decimal::percent(99),
            ),
        };

        let votes = MultipleChoiceVotes {
            vote_weights: vec![Uint128::new(9888889), Uint128::new(0), Uint128::new(0)],
        };
        let prop = create_proposal(
            &env.block,
            voting_strategy,
            votes,
            Uint128::new(10000000),
            true,
        );

        // Quorum was not met and expired, should reject
        assert!(!prop.is_passed(&env.block).unwrap());
        assert!(prop.is_rejected(&env.block).unwrap());
    }

    #[test]
    fn test_tricky_pass() {
        let env = mock_env();
        let voting_strategy = VotingStrategy::SingleChoice {
            quorum: voting::threshold::PercentageThreshold::Percent(
                cosmwasm_std::Decimal::from_ratio(7u32, 13u32),
            ),
        };
        let votes = MultipleChoiceVotes {
            vote_weights: vec![Uint128::new(7), Uint128::new(0), Uint128::new(6)],
        };
        let prop = create_proposal(
            &env.block,
            voting_strategy.clone(),
            votes.clone(),
            Uint128::new(13),
            true,
        );

        // Should pass if expired
        assert!(prop.is_passed(&env.block).unwrap());
        assert!(!prop.is_rejected(&env.block).unwrap());

        let prop = create_proposal(&env.block, voting_strategy, votes, Uint128::new(13), false);

        // Should pass if not expired
        assert!(prop.is_passed(&env.block).unwrap());
        assert!(!prop.is_rejected(&env.block).unwrap());
    }

    #[test]
    fn test_tricky_pass_majority() {
        let env = mock_env();
        let voting_strategy = VotingStrategy::SingleChoice {
            quorum: voting::threshold::PercentageThreshold::Majority {},
        };

        let votes = MultipleChoiceVotes {
            vote_weights: vec![Uint128::new(7), Uint128::new(0), Uint128::new(0)],
        };
        let prop = create_proposal(
            &env.block,
            voting_strategy.clone(),
            votes.clone(),
            Uint128::new(13),
            true,
        );

        // Should pass if majority voted
        assert!(prop.is_passed(&env.block).unwrap());
        assert!(!prop.is_rejected(&env.block).unwrap());

        let prop = create_proposal(&env.block, voting_strategy, votes, Uint128::new(14), true);

        // Shouldn't pass if only half voted
        assert!(!prop.is_passed(&env.block).unwrap());
        assert!(prop.is_rejected(&env.block).unwrap());
    }
}
