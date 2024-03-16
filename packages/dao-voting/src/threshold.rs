use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Decimal, Uint128};

use thiserror::Error;

/// The threshold of tokens that must be staked in order for this
/// voting module to be active. If this is not reached, this module
/// will response to `is_active` queries with false and proposal
/// modules which respect active thresholds will not allow the
/// creation of proposals.
#[cw_serde]
pub enum ActiveThreshold {
    /// The absolute number of tokens that must be staked for the
    /// module to be active.
    AbsoluteCount { count: Uint128 },
    /// The percentage of tokens that must be staked for the module to
    /// be active. Computed as `staked / total_supply`.
    Percentage { percent: Decimal },
}

#[cw_serde]
pub struct ActiveThresholdResponse {
    pub active_threshold: Option<ActiveThreshold>,
}

#[derive(Error, Debug, PartialEq, Eq)]
pub enum ActiveThresholdError {
    #[error("Absolute count threshold cannot be greater than the total token supply")]
    InvalidAbsoluteCount {},

    #[error("Active threshold percentage must be greater than 0 and not greater than 1")]
    InvalidActivePercentage {},

    #[error("Active threshold count must be greater than zero")]
    ZeroActiveCount {},
}

pub fn assert_valid_absolute_count_threshold(
    count: Uint128,
    supply: Uint128,
) -> Result<(), ActiveThresholdError> {
    if count.is_zero() {
        return Err(ActiveThresholdError::ZeroActiveCount {});
    }
    if count > supply {
        return Err(ActiveThresholdError::InvalidAbsoluteCount {});
    }
    Ok(())
}

pub fn assert_valid_percentage_threshold(percent: Decimal) -> Result<(), ActiveThresholdError> {
    if percent.is_zero() || percent > Decimal::one() {
        return Err(ActiveThresholdError::InvalidActivePercentage {});
    }
    Ok(())
}

#[derive(Error, Debug, PartialEq, Eq)]
pub enum ThresholdError {
    #[error("Required threshold cannot be zero")]
    ZeroThreshold {},

    #[error("Not possible to reach required (passing) threshold")]
    UnreachableThreshold {},
}

/// A percentage of voting power that must vote yes for a proposal to
/// pass. An example of why this is needed:
///
/// If a user specifies a 60% passing threshold, and there are 10
/// voters they likely expect that proposal to pass when there are 6
/// yes votes. This implies that the condition for passing should be
/// `vote_weights >= total_votes * threshold`.
///
/// With this in mind, how should a user specify that they would like
/// proposals to pass if the majority of voters choose yes? Selecting
/// a 50% passing threshold with those rules doesn't properly cover
/// that case as 5 voters voting yes out of 10 would pass the
/// proposal. Selecting 50.0001% or or some variation of that also
/// does not work as a very small yes vote which technically makes the
/// majority yes may not reach that threshold.
///
/// To handle these cases we provide both a majority and percent
/// option for all percentages. If majority is selected passing will
/// be determined by `yes > total_votes * 0.5`. If percent is selected
/// passing is determined by `yes >= total_votes * percent`.
///
/// In both of these cases a proposal with only abstain votes must
/// fail. This requires a special case passing logic.
#[cw_serde]
#[derive(Copy)]
pub enum PercentageThreshold {
    /// The majority of voters must vote yes for the proposal to pass.
    Majority {},
    /// A percentage of voting power >= percent must vote yes for the
    /// proposal to pass.
    Percent(Decimal),
}

/// The ways a proposal may reach its passing / failing threshold.
#[cw_serde]
pub enum Threshold {
    /// Declares a percentage of the total weight that must cast Yes
    /// votes in order for a proposal to pass.  See
    /// `ThresholdResponse::AbsolutePercentage` in the cw3 spec for
    /// details.
    AbsolutePercentage { percentage: PercentageThreshold },

    /// Declares a `quorum` of the total votes that must participate
    /// in the election in order for the vote to be considered at all.
    /// See `ThresholdResponse::ThresholdQuorum` in the cw3 spec for
    /// details.
    ThresholdQuorum {
        threshold: PercentageThreshold,
        quorum: PercentageThreshold,
    },

    /// An absolute number of votes needed for something to cross the
    /// threshold. Useful for multisig style voting.
    AbsoluteCount { threshold: Uint128 },
}

/// Asserts that the 0.0 < percent <= 1.0
fn validate_percentage(percent: &PercentageThreshold) -> Result<(), ThresholdError> {
    if let PercentageThreshold::Percent(percent) = percent {
        if percent.is_zero() {
            Err(ThresholdError::ZeroThreshold {})
        } else if *percent > Decimal::one() {
            Err(ThresholdError::UnreachableThreshold {})
        } else {
            Ok(())
        }
    } else {
        Ok(())
    }
}

/// Asserts that a quorum <= 1. Quorums may be zero, to enable plurality-style voting.
pub fn validate_quorum(quorum: &PercentageThreshold) -> Result<(), ThresholdError> {
    match quorum {
        PercentageThreshold::Majority {} => Ok(()),
        PercentageThreshold::Percent(quorum) => {
            if *quorum > Decimal::one() {
                Err(ThresholdError::UnreachableThreshold {})
            } else {
                Ok(())
            }
        }
    }
}

impl Threshold {
    /// Validates the threshold.
    ///
    /// - Quorums must never be over 100%.
    /// - Passing thresholds must never be over 100%, nor be 0%.
    /// - Absolute count thresholds must be non-zero.
    pub fn validate(&self) -> Result<(), ThresholdError> {
        match self {
            Threshold::AbsolutePercentage {
                percentage: percentage_needed,
            } => validate_percentage(percentage_needed),
            Threshold::ThresholdQuorum { threshold, quorum } => {
                validate_percentage(threshold)?;
                validate_quorum(quorum)
            }
            Threshold::AbsoluteCount { threshold } => {
                if threshold.is_zero() {
                    Err(ThresholdError::ZeroThreshold {})
                } else {
                    Ok(())
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! p {
        ($x:expr ) => {
            PercentageThreshold::Percent(Decimal::percent($x))
        };
    }

    #[test]
    fn test_threshold_validation() {
        let t = Threshold::AbsoluteCount {
            threshold: Uint128::zero(),
        };
        assert_eq!(t.validate().unwrap_err(), ThresholdError::ZeroThreshold {});

        let t = Threshold::AbsolutePercentage { percentage: p!(0) };
        assert_eq!(t.validate().unwrap_err(), ThresholdError::ZeroThreshold {});

        let t = Threshold::AbsolutePercentage {
            percentage: p!(101),
        };
        assert_eq!(
            t.validate().unwrap_err(),
            ThresholdError::UnreachableThreshold {}
        );

        let t = Threshold::AbsolutePercentage {
            percentage: p!(100),
        };
        t.validate().unwrap();

        let t = Threshold::ThresholdQuorum {
            threshold: p!(101),
            quorum: p!(0),
        };
        assert_eq!(
            t.validate().unwrap_err(),
            ThresholdError::UnreachableThreshold {}
        );

        let t = Threshold::ThresholdQuorum {
            threshold: p!(100),
            quorum: p!(0),
        };
        t.validate().unwrap();

        let t = Threshold::ThresholdQuorum {
            threshold: p!(100),
            quorum: p!(101),
        };
        assert_eq!(
            t.validate().unwrap_err(),
            ThresholdError::UnreachableThreshold {}
        );
    }
}
