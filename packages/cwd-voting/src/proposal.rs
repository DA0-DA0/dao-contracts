use cosmwasm_std::Addr;

use crate::status::Status;

/// Default limit for proposal pagination.
pub const DEFAULT_LIMIT: u64 = 30;
pub const MAX_PROPOSAL_SIZE: u64 = 30_000;

pub trait Proposal {
    fn proposer(&self) -> Addr;
    fn status(&self) -> Status;
}
