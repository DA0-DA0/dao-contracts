use cw_utils::Expiration;
use dao_voting::{
    status::Status,
    threshold::{PercentageThreshold, Threshold},
    voting::Votes,
};

pub(crate) fn v1_expiration_to_v2(v1: cw_utils_v1::Expiration) -> Expiration {
    match v1 {
        cw_utils_v1::Expiration::AtHeight(height) => Expiration::AtHeight(height),
        cw_utils_v1::Expiration::AtTime(time) => Expiration::AtTime(time),
        cw_utils_v1::Expiration::Never {} => Expiration::Never {},
    }
}

pub(crate) fn v1_percentage_threshold_to_v2(
    v1: voting_v1::PercentageThreshold,
) -> PercentageThreshold {
    match v1 {
        voting_v1::PercentageThreshold::Majority {} => PercentageThreshold::Majority {},
        voting_v1::PercentageThreshold::Percent(p) => PercentageThreshold::Percent(p),
    }
}

pub(crate) fn v1_threshold_to_v2(v1: voting_v1::Threshold) -> Threshold {
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

pub(crate) fn v1_status_to_v2(v1: voting_v1::Status) -> Status {
    match v1 {
        voting_v1::Status::Open => Status::Open,
        voting_v1::Status::Rejected => Status::Rejected,
        voting_v1::Status::Passed => Status::Passed,
        voting_v1::Status::Executed => Status::Executed,
        voting_v1::Status::Closed => Status::Closed,
    }
}

pub(crate) fn v1_votes_to_v2(v1: voting_v1::Votes) -> Votes {
    Votes {
        yes: v1.yes,
        no: v1.no,
        abstain: v1.abstain,
    }
}
