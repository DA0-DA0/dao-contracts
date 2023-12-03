use cosmwasm_schema::cw_serde;
use cosmwasm_std::{MessageInfo, StdError};
use cw_utils::Duration;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum VetoError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Proposal is {status} and thus is unable to be vetoed.")]
    InvalidProposalStatus { status: String },

    #[error("Early execution for timelocked proposals is not enabled. Proposal can not be executed before the timelock delay has expired.")]
    NoEarlyExecute {},

    #[error("Veto is not enabled for this contract.")]
    NoVetoConfiguration {},

    #[error("Vetoing before a proposal passes is not enabled.")]
    NoVetoBeforePassed {},

    #[error("The proposal is timelocked and cannot be executed.")]
    Timelocked {},

    #[error("The veto timelock duration has expired.")]
    TimelockExpired {},

    #[error("Only vetoer can veto a proposal.")]
    Unauthorized {},

    #[error("Zero timelock duration is only permitted with veto_before_passed")]
    DurationMisconfiguration {},
}

#[cw_serde]
pub struct VetoConfig {
    /// The time duration to delay proposal execution for.
    pub timelock_duration: Duration,
    /// The address able to veto proposals.
    pub vetoer: String,
    /// Whether or not the vetoer can execute a proposal early before the
    /// timelock duration has expired
    pub early_execute: bool,
    /// Whether or not the vetoer can veto a proposal before it passes.
    pub veto_before_passed: bool,
}

impl VetoConfig {
    pub fn validate(&self) -> Result<(), VetoError> {
        let timelock_duration = match self.timelock_duration {
            Duration::Height(h) => h,
            Duration::Time(t) => t,
        };
        if timelock_duration == 0 && !self.veto_before_passed {
            Err(VetoError::DurationMisconfiguration {})
        } else {
            Ok(())
        }
    }

    /// Whether early execute is enabled
    pub fn check_early_execute_enabled(&self) -> Result<(), VetoError> {
        if self.early_execute {
            Ok(())
        } else {
            Err(VetoError::NoEarlyExecute {})
        }
    }

    /// Checks whether the message sender is the vetoer.
    pub fn check_is_vetoer(&self, info: &MessageInfo) -> Result<(), VetoError> {
        if self.vetoer == info.sender {
            Ok(())
        } else {
            Err(VetoError::Unauthorized {})
        }
    }

    /// Checks whether veto_before_passed is enabled, errors if not
    pub fn check_veto_before_passed_enabled(&self) -> Result<(), VetoError> {
        if self.veto_before_passed {
            Ok(())
        } else {
            Err(VetoError::NoVetoBeforePassed {})
        }
    }
}
