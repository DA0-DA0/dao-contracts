use crate::{proposal::MultipleChoiceProposal, voting_strategy::VotingStrategy, ContractError};
use cosmwasm_std::{Addr, CosmosMsg, Empty, Uint128};
use cw_storage_plus::{Item, Map};
use cw_utils::Duration;
use indexable_hooks::Hooks;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use voting::{deposit::CheckedDepositInfo, voting::MultipleChoiceVote};

pub const MAX_NUM_CHOICES: u32 = 10;
const NONE_OPTION_DESCRIPTION: &str = "None of the above";

/// The governance module's configuration.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    /// The threshold a proposal must reach to complete.
    pub voting_strategy: VotingStrategy,
    /// The minimum amount of time a proposal must be open before
    /// passing. A proposal may fail before this amount of time has
    /// elapsed, but it will not pass. This can be useful for
    /// preventing governance attacks wherein an attacker aquires a
    /// large number of tokens and forces a proposal through.
    pub min_voting_period: Option<Duration>,
    /// The default maximum amount of time a proposal may be voted on
    /// before expiring.
    pub max_voting_period: Duration,
    /// If set to true only members may execute passed
    /// proposals. Otherwise, any address may execute a passed
    /// proposal.
    pub only_members_execute: bool,
    /// The address of the DAO that this governance module is
    /// associated with.
    pub dao: Addr,
    /// Information about the depost required to create a
    /// proposal. None if no deposit is required, Some otherwise.
    pub deposit_info: Option<CheckedDepositInfo>,
}

/// Information about a vote that was cast.
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, JsonSchema, Debug)]
pub struct VoteInfo {
    /// The address that voted.
    pub voter: Addr,
    /// Position on the vote.
    pub vote: MultipleChoiceVote,
    /// The voting power behind the vote.
    pub power: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub enum MultipleChoiceOptionType {
    /// Choice that represents selecting none of the options; still counts toward quorum
    /// and allows proposals with all bad options to be voted against.
    None,
    Standard,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MultipleChoiceOptions {
    pub options: Vec<MultipleChoiceOption>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CheckedMultipleChoiceOptions {
    pub options: Vec<CheckedMultipleChoiceOption>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MultipleChoiceOption {
    pub description: String,
    pub msgs: Option<Vec<CosmosMsg<Empty>>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CheckedMultipleChoiceOption {
    // This is the index of the option in both the vote_weights and proposal.choices vectors.
    // Workaround due to not being able to use HashMaps in Cosmwasm.
    pub index: u32,
    pub option_type: MultipleChoiceOptionType,
    pub description: String,
    pub msgs: Option<Vec<CosmosMsg<Empty>>>,
    pub vote_count: Uint128,
}

impl MultipleChoiceOptions {
    pub fn into_checked(self) -> Result<CheckedMultipleChoiceOptions, ContractError> {
        if self.options.len() < 2 || self.options.len() > MAX_NUM_CHOICES as usize {
            return Err(ContractError::WrongNumberOfChoices {});
        }

        let mut checked_options: Vec<CheckedMultipleChoiceOption> =
            Vec::with_capacity(self.options.len() + 1);

        // Iterate through choices and save the index and option type for each
        self.options
            .into_iter()
            .enumerate()
            .for_each(|(idx, choice)| {
                let checked_option = CheckedMultipleChoiceOption {
                    index: idx as u32,
                    option_type: MultipleChoiceOptionType::Standard,
                    description: choice.description,
                    msgs: choice.msgs,
                    vote_count: Uint128::zero(),
                };
                checked_options.push(checked_option);
            });

        // Add a "None of the above" option, required for every multiple choice proposal.
        let none_option = CheckedMultipleChoiceOption {
            index: (checked_options.capacity() - 1) as u32,
            option_type: MultipleChoiceOptionType::None,
            description: NONE_OPTION_DESCRIPTION.to_string(),
            msgs: None,
            vote_count: Uint128::zero(),
        };

        checked_options.push(none_option);

        let options = CheckedMultipleChoiceOptions {
            options: checked_options,
        };
        Ok(options)
    }
}

// we cast a ballot with our chosen vote and a given weight
// stored under the key that voted
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, JsonSchema, Debug)]
pub struct Ballot {
    /// The amount of voting power behind the vote.
    pub power: Uint128,
    /// The position.
    pub vote: MultipleChoiceVote,
}

pub const CONFIG: Item<Config> = Item::new("config");
pub const PROPOSAL_COUNT: Item<u64> = Item::new("proposal_count");
pub const PROPOSALS: Map<u64, MultipleChoiceProposal> = Map::new("proposals");
pub const BALLOTS: Map<(u64, Addr), Ballot> = Map::new("ballots");
pub const PROPOSAL_HOOKS: Hooks = Hooks::new("proposal_hooks");
pub const VOTE_HOOKS: Hooks = Hooks::new("vote_hooks");

mod tests {

    #[test]
    fn test_into_checked() {
        let options = vec![
            super::MultipleChoiceOption {
                description: "multiple choice option 1".to_string(),
                msgs: None,
            },
            super::MultipleChoiceOption {
                description: "multiple choice option 2".to_string(),
                msgs: None,
            },
        ];

        let mc_options = super::MultipleChoiceOptions { options };

        let checked_mc_options = mc_options.into_checked().unwrap();
        assert_eq!(checked_mc_options.options.len(), 3);
        assert_eq!(
            checked_mc_options.options[0].option_type,
            super::MultipleChoiceOptionType::Standard
        );
        assert_eq!(
            checked_mc_options.options[0].description,
            "multiple choice option 1",
        );
        assert_eq!(
            checked_mc_options.options[1].option_type,
            super::MultipleChoiceOptionType::Standard
        );
        assert_eq!(
            checked_mc_options.options[1].description,
            "multiple choice option 2",
        );
        assert_eq!(
            checked_mc_options.options[2].option_type,
            super::MultipleChoiceOptionType::None
        );
        assert_eq!(
            checked_mc_options.options[2].description,
            super::NONE_OPTION_DESCRIPTION,
        );
    }

    #[test]
    fn test_into_checked_wrong_num_choices() {
        let options = vec![super::MultipleChoiceOption {
            description: "multiple choice option 1".to_string(),
            msgs: None,
        }];

        let mc_options = super::MultipleChoiceOptions { options };
        let res = mc_options.into_checked();
        assert!(matches!(
            res.unwrap_err(),
            super::ContractError::WrongNumberOfChoices {}
        ))
    }
}
