use cosmwasm_schema::cw_serde;
use cosmwasm_std::{MessageInfo, StdError};
use cw_utils::Duration;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum TimelockError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Proposal is {status}, this proposal status is unable to be vetoed.")]
    InvalidProposalStatus { status: String },

    #[error("Early execution for timelocked proposals is not enabled. Proposal can not be executed before the timelock delay has expired.")]
    NoEarlyExecute {},

    #[error("Timelock is not configured for this contract. Veto not enabled.")]
    NoTimelock {},

    #[error("Vetoing before a proposal passes is not enabled.")]
    NoVetoBeforePassed {},

    #[error("The proposal is time locked and cannot be executed.")]
    Timelocked {},

    #[error("The timelock duration has expired.")]
    TimelockExpired {},

    #[error("Only vetoer can veto a proposal.")]
    Unauthorized {},
}

#[cw_serde]
pub struct Timelock {
    /// The time duration to delay proposal execution for.
    pub delay: Duration,
    /// The address able to veto proposals.
    pub vetoer: String,
    /// Whether or not the vetoer can execute a proposal early before the
    /// timelock duration has expired
    pub early_execute: bool,
    /// Whether or not the vetoer can veto a proposal before it passes.
    pub veto_before_passed: bool,
}

impl Timelock {
    /// Whether early execute is enabled
    pub fn check_early_execute_enabled(&self) -> Result<(), TimelockError> {
        if self.early_execute {
            Ok(())
        } else {
            Err(TimelockError::NoEarlyExecute {})
        }
    }

    /// Checks whether the message sender is the vetoer.
    pub fn check_is_vetoer(&self, info: &MessageInfo) -> Result<(), TimelockError> {
        if self.vetoer == info.sender {
            Ok(())
        } else {
            Err(TimelockError::Unauthorized {})
        }
    }

    /// Checks whether veto_before_passed is enabled, errors if not
    pub fn check_veto_before_passed_enabled(&self) -> Result<(), TimelockError> {
        if self.veto_before_passed {
            Ok(())
        } else {
            Err(TimelockError::NoVetoBeforePassed {})
        }
    }
}
