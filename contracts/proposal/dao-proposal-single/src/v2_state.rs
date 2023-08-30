//! Helper methods for migrating from v2 to v3 state. These will need
//! to be updated when we bump our CosmWasm version for v3.

use cw_utils::{Duration, Expiration};
use dao_voting::{
    status::Status,
    threshold::{PercentageThreshold, Threshold},
    voting::Votes,
};

pub fn v2_percentage_threshold_to_v3(
    v2: voting_v2::threshold::PercentageThreshold,
) -> PercentageThreshold {
    match v2 {
        voting_v2::threshold::PercentageThreshold::Majority {} => PercentageThreshold::Majority {},
        voting_v2::threshold::PercentageThreshold::Percent(p) => PercentageThreshold::Percent(p),
    }
}

pub fn v2_threshold_to_v3(v2: voting_v2::threshold::Threshold) -> Threshold {
    match v2 {
        voting_v2::threshold::Threshold::AbsolutePercentage { percentage } => {
            Threshold::AbsolutePercentage {
                percentage: v2_percentage_threshold_to_v3(percentage),
            }
        }
        voting_v2::threshold::Threshold::ThresholdQuorum { threshold, quorum } => {
            Threshold::ThresholdQuorum {
                threshold: v2_percentage_threshold_to_v3(threshold),
                quorum: v2_percentage_threshold_to_v3(quorum),
            }
        }
        voting_v2::threshold::Threshold::AbsoluteCount { threshold } => {
            Threshold::AbsoluteCount { threshold }
        }
    }
}

pub fn v2_duration_to_v3(v2: cw_utils_v2::Duration) -> Duration {
    match v2 {
        cw_utils_v2::Duration::Height(height) => Duration::Height(height),
        cw_utils_v2::Duration::Time(time) => Duration::Time(time),
    }
}

pub fn v2_expiration_to_v3(v2: cw_utils_v2::Expiration) -> Expiration {
    match v2 {
        cw_utils_v2::Expiration::AtHeight(height) => Expiration::AtHeight(height),
        cw_utils_v2::Expiration::AtTime(time) => Expiration::AtTime(time),
        cw_utils_v2::Expiration::Never {} => Expiration::Never {},
    }
}

pub fn v2_votes_to_v3(v2: voting_v2::voting::Votes) -> Votes {
    Votes {
        yes: v2.yes,
        no: v2.no,
        abstain: v2.abstain,
    }
}

pub fn v2_status_to_v3(v2: voting_v2::status::Status) -> Status {
    match v2 {
        voting_v2::status::Status::Open => Status::Open,
        voting_v2::status::Status::Rejected => Status::Rejected,
        voting_v2::status::Status::Passed => todo!(),
        voting_v2::status::Status::Executed => Status::Executed,
        voting_v2::status::Status::Closed => Status::Closed,
        voting_v2::status::Status::ExecutionFailed => Status::ExecutionFailed,
    }
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{Decimal, Timestamp, Uint128};

    use super::*;

    #[test]
    fn test_percentage_conversion() {
        assert_eq!(
            v2_percentage_threshold_to_v3(voting_v2::threshold::PercentageThreshold::Majority {}),
            PercentageThreshold::Majority {}
        );
        assert_eq!(
            v2_percentage_threshold_to_v3(voting_v2::threshold::PercentageThreshold::Percent(
                Decimal::percent(80)
            )),
            PercentageThreshold::Percent(Decimal::percent(80))
        )
    }

    #[test]
    fn test_duration_conversion() {
        assert_eq!(
            v2_duration_to_v3(cw_utils_v2::Duration::Height(100)),
            Duration::Height(100)
        );
        assert_eq!(
            v2_duration_to_v3(cw_utils_v2::Duration::Time(100)),
            Duration::Time(100)
        );
    }

    #[test]
    fn test_expiration_conversion() {
        assert_eq!(
            v2_expiration_to_v3(cw_utils_v2::Expiration::AtHeight(100)),
            Expiration::AtHeight(100)
        );
        assert_eq!(
            v2_expiration_to_v3(cw_utils_v2::Expiration::AtTime(Timestamp::from_seconds(
                100
            ))),
            Expiration::AtTime(Timestamp::from_seconds(100))
        );
        assert_eq!(
            v2_expiration_to_v3(cw_utils_v2::Expiration::Never {}),
            Expiration::Never {}
        );
    }

    #[test]
    fn test_threshold_conversion() {
        assert_eq!(
            v2_threshold_to_v3(voting_v2::threshold::Threshold::AbsoluteCount {
                threshold: Uint128::new(10)
            }),
            Threshold::AbsoluteCount {
                threshold: Uint128::new(10)
            }
        );
        assert_eq!(
            v2_threshold_to_v3(voting_v2::threshold::Threshold::AbsolutePercentage {
                percentage: voting_v2::threshold::PercentageThreshold::Majority {}
            }),
            Threshold::AbsolutePercentage {
                percentage: PercentageThreshold::Majority {}
            }
        );
        assert_eq!(
            v2_threshold_to_v3(voting_v2::threshold::Threshold::ThresholdQuorum {
                threshold: voting_v2::threshold::PercentageThreshold::Majority {},
                quorum: voting_v2::threshold::PercentageThreshold::Percent(Decimal::percent(20))
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
                    v2_status_to_v3({
                        use voting_v2::status::Status;
                        $x
                    }),
                    $x
                )
            };
        }

        status_conversion!(Status::Open);
        status_conversion!(Status::Closed);
        status_conversion!(Status::Executed);
        status_conversion!(Status::Rejected);
        status_conversion!(Status::ExecutionFailed);
        // TODO test passed status conversion
        // status_conversion!(Status::Passed);
    }
}
