use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use voting::threshold::{validate_quorum, PercentageThreshold, ThresholdError};

/// Determines the way votes are counted.
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum VotingStrategy {
    SingleChoice { quorum: PercentageThreshold },
}

impl VotingStrategy {
    pub fn validate(&self) -> Result<(), ThresholdError> {
        match self {
            VotingStrategy::SingleChoice { quorum } => validate_quorum(quorum),
        }
    }

    pub fn get_quorum(&self) -> PercentageThreshold {
        match self {
            VotingStrategy::SingleChoice { quorum } => *quorum,
        }
    }
}
