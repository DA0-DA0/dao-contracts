use cosmwasm_schema::cw_serde;
use cw_utils::Duration;
use dao_voting::threshold::PercentageThreshold;

#[cw_serde]
pub struct Config {
    pub quorum: PercentageThreshold,
    pub voting_period: Duration,
    pub min_voting_period: Option<Duration>,
}
