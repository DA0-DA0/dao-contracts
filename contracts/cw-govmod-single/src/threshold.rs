use cosmwasm_std::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ContractError;

/// The ways a proposal may reach its passing / failing threshold.
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Threshold {
    /// Declares a percentage of the total weight that must cast Yes
    /// votes in order for a proposal to pass.  See
    /// `ThresholdResponse::AbsolutePercentage` in the cw3 spec for
    /// details.
    AbsolutePercentage { percentage: Decimal },

    /// Declares a `quorum` of the total votes that must participate
    /// in the election in order for the vote to be considered at all.
    /// See `ThresholdResponse::ThresholdQuorum` in the cw3 spec for
    /// details.
    ThresholdQuorum { threshold: Decimal, quorum: Decimal },
}

/// Asserts that the 0.0 < percent <= 1.0
fn validate_percentage(percent: &Decimal) -> Result<(), ContractError> {
    if percent.is_zero() {
        Err(ContractError::ZeroThreshold {})
    } else if *percent > Decimal::one() {
        Err(ContractError::UnreachableThreshold {})
    } else {
        Ok(())
    }
}

impl Threshold {
    /// returns error if this is an unreachable value,
    /// given a total weight of all members in the group
    pub fn validate(&self) -> Result<(), ContractError> {
        match self {
            Threshold::AbsolutePercentage {
                percentage: percentage_needed,
            } => validate_percentage(percentage_needed),
            Threshold::ThresholdQuorum { threshold, quorum } => {
                validate_percentage(threshold)?;
                validate_percentage(quorum)
            }
        }
    }
}
