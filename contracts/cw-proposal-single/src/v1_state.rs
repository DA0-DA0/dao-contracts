//! Helper methods for migrating from v1 to v2 state. These will need
//! to be updated when we bump our CosmWasm version for v2.

use voting::{
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
