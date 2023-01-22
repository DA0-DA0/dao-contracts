use cosmwasm_schema::cw_serde;
use cw_utils::Duration;
use dao_voting::{
    threshold::{validate_quorum, PercentageThreshold},
    voting::validate_voting_period,
};

use crate::ContractError;

#[cw_serde]
pub struct UncheckedConfig {
    pub quorum: PercentageThreshold,
    pub voting_period: Duration,
    pub min_voting_period: Option<Duration>,
    pub close_proposals_on_execution_failure: bool,
}

#[cw_serde]
pub(crate) struct Config {
    pub quorum: PercentageThreshold,
    pub voting_period: Duration,
    pub min_voting_period: Option<Duration>,
    pub close_proposals_on_execution_failure: bool,
}

impl UncheckedConfig {
    pub(crate) fn into_checked(self) -> Result<Config, ContractError> {
        validate_quorum(&self.quorum)?;
        let (min_voting_period, voting_period) =
            validate_voting_period(self.min_voting_period, self.voting_period)?;
        Ok(Config {
            quorum: self.quorum,
            close_proposals_on_execution_failure: self.close_proposals_on_execution_failure,
            voting_period,
            min_voting_period,
        })
    }
}
