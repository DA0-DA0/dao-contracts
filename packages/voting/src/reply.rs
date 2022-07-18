/// Masks for reply id
const FAILED_PROPOSAL_EXECUTION_MASK: u64 = 0b00;
const FAILED_PROPOSAL_HOOK_MASK: u64 = 0b01;
const FAILED_VOTE_HOOK_MASK: u64 = 0b11;

const BITS_RESERVED_FOR_REPLY_TYPE: u8 = 2;
const REPLY_TYPE_MASK: u64 = (1 << BITS_RESERVED_FOR_REPLY_TYPE) - 1;

/// Since we can only pass `id`, and we need to perform different actions in reply,
/// we decided to take few bits to identify "Reply Type".
/// See <https://github.com/DA0-DA0/dao-contracts/pull/385#discussion_r916324843>
pub enum TaggedReplyId {
    FailedProposalExecution(u64),
    FailedProposalHook(u64),
    FailedVoteHook(u64),
}

impl TaggedReplyId {
    /// Takes `Reply.id` and returns tagged version of it,
    /// depending on a first few bits.
    ///
    /// We know it costs extra pattern match, but cleaner code in `reply` Methods
    pub fn new(id: u64) -> Self {
        let reply_type = id & REPLY_TYPE_MASK;
        let id = id >> BITS_RESERVED_FOR_REPLY_TYPE;
        match reply_type {
            FAILED_PROPOSAL_EXECUTION_MASK => TaggedReplyId::FailedProposalExecution(id),
            FAILED_PROPOSAL_HOOK_MASK => TaggedReplyId::FailedProposalHook(id),
            FAILED_VOTE_HOOK_MASK => TaggedReplyId::FailedVoteHook(id),
            _ => unreachable!(),
        }
    }
}

/// This function can drop bits, if you have more than `u(64-[`BITS_RESERVED_FOR_REPLY_TYPE`])` proposals.
pub fn mask_proposal_execution_proposal_id(proposal_id: u64) -> u64 {
    FAILED_PROPOSAL_EXECUTION_MASK | (proposal_id << BITS_RESERVED_FOR_REPLY_TYPE)
}

pub fn mask_proposal_hook_index(index: u64) -> u64 {
    FAILED_PROPOSAL_HOOK_MASK | (index << BITS_RESERVED_FOR_REPLY_TYPE)
}

pub fn mask_vote_hook_index(index: u64) -> u64 {
    FAILED_VOTE_HOOK_MASK | (index << BITS_RESERVED_FOR_REPLY_TYPE)
}
