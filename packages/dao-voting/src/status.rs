use cosmwasm_schema::cw_serde;
use cosmwasm_std::Timestamp;

// TODO review these if we are going to go through the pain of changing them
#[cw_serde]
pub enum Status {
    /// The proposal is open for voting.
    Open,
    /// The proposal has been rejected.
    Rejected { at_time: Timestamp },
    /// The proposal has been passed but has not been executed.
    Passed { at_time: Timestamp },
    /// The proposal has been passed and executed.
    Executed { tx_hash: String },
    /// The proposal has failed or expired and has been closed. A
    /// proposal deposit refund has been issued if applicable.
    Closed { at_time: Timestamp },
    /// The proposal's execution failed.
    ExecutionFailed { err: String },
    /// The proposal has been vetoed.
    Vetoed { rational: Option<String> },
}

impl std::fmt::Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Status::Open => write!(f, "open"),
            Status::Rejected { at_time } => write!(f, "rejected {:?}", at_time),
            Status::Passed { at_time } => write!(f, "passed {:?}", at_time),
            Status::Executed { tx_hash } => write!(f, "executed {:?}", tx_hash),
            Status::Closed { at_time } => write!(f, "closed {:?}", at_time),
            Status::ExecutionFailed { err } => write!(f, "execution_failed {:?}", err),
            Status::Vetoed { rational } => write!(f, "vetoed {:?}", rational),
        }
    }
}
