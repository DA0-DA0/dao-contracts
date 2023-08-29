use cosmwasm_schema::cw_serde;
use cosmwasm_std::{MessageInfo, Timestamp};

#[cw_serde]
pub struct Timelock {
    /// The time duration to delay proposal execution for
    pub duration: Timestamp,
    /// The account able to veto proposals.
    pub vetoer: String,
}

impl Timelock {
    /// TODO need to refactor as we don't have timestamp the prop passed?
    /// Takes two timestamps and returns true if the proposal is locked or not.
    pub fn is_locked(&self, proposal_passed: Timestamp, at_time: Timestamp) -> bool {
        proposal_passed.seconds() + self.duration.seconds() < at_time.seconds()
    }

    /// Checks whether the message sender is the vetoer.
    pub fn is_vetoer(&self, info: MessageInfo) -> bool {
        self.vetoer == info.sender.to_string()
    }
}
