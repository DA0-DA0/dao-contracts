use cosmwasm_std::Addr;

use crate::{deposit::CheckedDepositInfo, status::Status};

/// Default limit for proposal pagination.
pub const DEFAULT_LIMIT: u64 = 30;
pub const MAX_PROPOSAL_SIZE: u64 = 30_000;

/// Masks for reply id
pub const FAILED_PROPOSAL_EXECUTION_MASK: u64 = 0b00;
pub const FAILED_PROPOSAL_HOOK_MASK: u64 = 0b01;
pub const FAILED_VOTE_HOOK_MASK: u64 = 0b11;

pub const BITS_RESERVED_FOR_REPLY_TYPE: u8 = 2;
pub const REPLY_TYPE_MASK: u64 = (1 << BITS_RESERVED_FOR_REPLY_TYPE) - 1;

pub trait Proposal {
    fn proposer(&self) -> Addr;
    fn deposit_info(&self) -> Option<CheckedDepositInfo>;
    fn status(&self) -> Status;
}
