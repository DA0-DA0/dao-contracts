use cw_utils::Duration;
use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum UnstakingDurationError {
    #[error("Invalid unstaking duration, unstaking duration cannot be 0")]
    InvalidUnstakingDuration {},
}

pub fn validate_duration(duration: Option<Duration>) -> Result<(), UnstakingDurationError> {
    if let Some(unstaking_duration) = duration {
        match unstaking_duration {
            Duration::Height(height) => {
                if height == 0 {
                    return Err(UnstakingDurationError::InvalidUnstakingDuration {});
                }
            }
            Duration::Time(time) => {
                if time == 0 {
                    return Err(UnstakingDurationError::InvalidUnstakingDuration {});
                }
            }
        }
    }
    Ok(())
}
