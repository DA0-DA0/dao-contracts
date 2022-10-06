use cosmwasm_std::Uint128;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use voting::threshold::{validate_quorum, validate_count, PercentageThreshold, ThresholdError};

/// Determines the way votes are counted.
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum MultipleProposalThreshold {
    Percentage { quorum: PercentageThreshold },
    Absoulute { threshold: Uint128 },
}


/// Determines the way votes are counted.
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum VotingStrategy {
    SingleChoice(MultipleProposalThreshold),
}

impl VotingStrategy {
    pub fn validate(&self) -> Result<(), ThresholdError> {
        match self {
            VotingStrategy::SingleChoice(thresold) => {
                match thresold {
                    MultipleProposalThreshold::Absoulute { threshold } => validate_count(threshold),
                    MultipleProposalThreshold::Percentage{ quorum } => validate_quorum(quorum),
                }    
            }
        }   
    }
}
