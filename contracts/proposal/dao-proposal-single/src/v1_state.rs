//! Helper methods for migrating from v1 to v2 state. These will need
//! to be updated when we bump our CosmWasm version for v2.

use cw_utils::{Duration, Expiration};
use dao_voting::{
    status::Status,
    threshold::{PercentageThreshold, Threshold},
    voting::Votes,
};

pub fn v1_percentage_threshold_to_v2(v1: voting_v1::PercentageThreshold) -> PercentageThreshold {
    match v1 {
        voting_v1::PercentageThreshold::Majority {} => PercentageThreshold::Majority {},
        voting_v1::PercentageThreshold::Percent(p) => PercentageThreshold::Percent(p),
    }
}

pub fn v1_threshold_to_v2(v1: voting_v1::Threshold) -> Threshold {
    match v1 {
        voting_v1::Threshold::AbsolutePercentage { percentage } => Threshold::AbsolutePercentage {
            percentage: v1_percentage_threshold_to_v2(percentage),
        },
        voting_v1::Threshold::ThresholdQuorum { threshold, quorum } => Threshold::ThresholdQuorum {
            threshold: v1_percentage_threshold_to_v2(threshold),
            quorum: v1_percentage_threshold_to_v2(quorum),
        },
        voting_v1::Threshold::AbsoluteCount { threshold } => Threshold::AbsoluteCount { threshold },
    }
}

pub fn v1_duration_to_v2(v1: cw_utils_v1::Duration) -> Duration {
    match v1 {
        cw_utils_v1::Duration::Height(height) => Duration::Height(height),
        cw_utils_v1::Duration::Time(time) => Duration::Time(time),
    }
}

pub fn v1_expiration_to_v2(v1: cw_utils_v1::Expiration) -> Expiration {
    match v1 {
        cw_utils_v1::Expiration::AtHeight(height) => Expiration::AtHeight(height),
        cw_utils_v1::Expiration::AtTime(time) => Expiration::AtTime(time),
        cw_utils_v1::Expiration::Never {} => Expiration::Never {},
    }
}

pub fn v1_votes_to_v2(v1: voting_v1::Votes) -> Votes {
    Votes {
        yes: v1.yes,
        no: v1.no,
        abstain: v1.abstain,
    }
}

pub fn v1_status_to_v2(v1: voting_v1::Status) -> Status {
    match v1 {
        voting_v1::Status::Open => Status::Open,
        voting_v1::Status::Rejected => Status::Rejected,
        voting_v1::Status::Passed => Status::Passed,
        voting_v1::Status::Executed => Status::Executed,
        voting_v1::Status::Closed => Status::Closed,
    }
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{Decimal, Timestamp, Uint128};

    use super::*;

    #[test]
    fn test_percentage_conversion() {
        assert_eq!(
            v1_percentage_threshold_to_v2(voting_v1::PercentageThreshold::Majority {}),
            PercentageThreshold::Majority {}
        );
        assert_eq!(
            v1_percentage_threshold_to_v2(voting_v1::PercentageThreshold::Percent(
                Decimal::percent(80)
            )),
            PercentageThreshold::Percent(Decimal::percent(80))
        )
    }

    #[test]
    fn test_duration_conversion() {
        assert_eq!(
            v1_duration_to_v2(cw_utils_v1::Duration::Height(100)),
            Duration::Height(100)
        );
        assert_eq!(
            v1_duration_to_v2(cw_utils_v1::Duration::Time(100)),
            Duration::Time(100)
        );
    }

    #[test]
    fn test_expiration_conversion() {
        assert_eq!(
            v1_expiration_to_v2(cw_utils_v1::Expiration::AtHeight(100)),
            Expiration::AtHeight(100)
        );
        assert_eq!(
            v1_expiration_to_v2(cw_utils_v1::Expiration::AtTime(Timestamp::from_seconds(
                100
            ))),
            Expiration::AtTime(Timestamp::from_seconds(100))
        );
        assert_eq!(
            v1_expiration_to_v2(cw_utils_v1::Expiration::Never {}),
            Expiration::Never {}
        );
    }

    #[test]
    fn test_threshold_conversion() {
        assert_eq!(
            v1_threshold_to_v2(voting_v1::Threshold::AbsoluteCount {
                threshold: Uint128::new(10)
            }),
            Threshold::AbsoluteCount {
                threshold: Uint128::new(10)
            }
        );
        assert_eq!(
            v1_threshold_to_v2(voting_v1::Threshold::AbsolutePercentage {
                percentage: voting_v1::PercentageThreshold::Majority {}
            }),
            Threshold::AbsolutePercentage {
                percentage: PercentageThreshold::Majority {}
            }
        );
        assert_eq!(
            v1_threshold_to_v2(voting_v1::Threshold::ThresholdQuorum {
                threshold: voting_v1::PercentageThreshold::Majority {},
                quorum: voting_v1::PercentageThreshold::Percent(Decimal::percent(20))
            }),
            Threshold::ThresholdQuorum {
                threshold: PercentageThreshold::Majority {},
                quorum: PercentageThreshold::Percent(Decimal::percent(20))
            }
        );
    }

    #[test]
    fn test_status_conversion() {
        macro_rules! status_conversion {
            ($x:expr) => {
                assert_eq!(
                    v1_status_to_v2({
                        use voting_v1::Status;
                        $x
                    }),
                    $x
                )
            };
        }

        status_conversion!(Status::Open);
        status_conversion!(Status::Closed);
        status_conversion!(Status::Executed);
        status_conversion!(Status::Rejected)
    }
}
