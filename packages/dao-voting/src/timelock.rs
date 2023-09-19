use cosmwasm_schema::cw_serde;
use cosmwasm_std::{MessageInfo, StdError, Timestamp};
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum TimelockError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Early execution for timelocked proposals is not enabled. Proposal can not be executed before the timelock delay has expired.")]
    NoEarlyExecute {},

    #[error("Timelock is not configured for this contract. Veto not enabled.")]
    NoTimelock {},

    #[error("The proposal is time locked and cannot be executed.")]
    Timelocked {},

    #[error("The timelock duration has expired.")]
    TimelockedExpired {},

    #[error("Only vetoer can veto a proposal.")]
    Unauthorized {},
}

#[cw_serde]
pub struct Timelock {
    /// The time duration to delay proposal execution for
    pub delay: Timestamp,
    /// The account able to veto proposals.
    pub vetoer: String,
    /// Whether or not the vetoer can excute a proposal early before the
    /// timelock duration has expired
    pub early_execute: bool,
}

impl Timelock {
    /// Calculate the expiration time for the timelock
    pub fn calculate_timelock_expiration(&self, current_time: Timestamp) -> Timestamp {
        Timestamp::from_seconds(current_time.seconds() + self.delay.seconds())
    }

    /// Whether early execute is enabled
    pub fn check_early_excute_enabled(&self) -> Result<(), TimelockError> {
        if self.early_execute {
            Ok(())
        } else {
            Err(TimelockError::NoEarlyExecute {})
        }
    }

    /// Takes two timestamps and returns true if the proposal is locked or not.
    pub fn check_is_locked(
        &self,
        current_time: Timestamp,
        expires: Timestamp,
    ) -> Result<(), TimelockError> {
        if current_time.seconds() > expires.seconds() {
            Ok(())
        } else {
            Err(TimelockError::Timelocked {})
        }
    }

    /// Checks whether the message sender is the vetoer.
    pub fn check_is_vetoer(&self, info: &MessageInfo) -> Result<(), TimelockError> {
        if self.vetoer == info.sender.to_string() {
            Ok(())
        } else {
            Err(TimelockError::Unauthorized {})
        }
    }
}
