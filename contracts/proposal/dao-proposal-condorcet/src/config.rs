use cw_utils::Duration;
use dao_voting::threshold::PercentageThreshold;

pub struct Config {
    quorum: PercentageThreshold,
    voting_period: Duration,
}
