use cw_core_macros::limit_variant_count;

const FAILED_PROPOSAL_EXECUTION_MASK: u64 = 0b00;
const FAILED_PROPOSAL_HOOK_MASK: u64 = 0b01;
const FAILED_VOTE_HOOK_MASK: u64 = 0b10;
const PRE_PROPOSE_MODULE_INSTANTIATION_MASK: u64 = 0b11;

const BITS_RESERVED_FOR_REPLY_TYPE: u8 = 2;
const REPLY_TYPE_MASK: u64 = (1 << BITS_RESERVED_FOR_REPLY_TYPE) - 1;

/// Since we can only pass `id`, and we need to perform different actions in reply,
/// we decided to take few bits to identify "Reply Type".
/// See <https://github.com/DA0-DA0/dao-contracts/pull/385#discussion_r916324843>
// Limit variant count to `2 ** BITS_RESERVED_FOR_REPLY_TYPE`. This
// must be manually updated if additional bits are allocated as
// constexpr and procedural macros are seprate in the rust compiler.
#[limit_variant_count(4)]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug, PartialEq, Eq))]
pub enum TaggedReplyId {
    /// Fired when a proposal's execution fails.
    FailedProposalExecution(u64),
    /// Fired when a proposal hook's execution fails.
    FailedProposalHook(u64),
    /// Fired when a vote hook's execution fails.
    FailedVoteHook(u64),
    /// Fired when a pre-propose module is successfully instantiated.
    PreProposeModuleInstantiation,
}

impl TaggedReplyId {
    /// Takes `Reply.id` and returns tagged version of it,
    /// depending on a first few bits.
    ///
    /// We know it costs extra pattern match, but cleaner code in `reply` Methods
    pub fn new(id: u64) -> Result<Self, error::TagError> {
        let reply_type = id & REPLY_TYPE_MASK;
        let id_after_shift = id >> BITS_RESERVED_FOR_REPLY_TYPE;
        match reply_type {
            FAILED_PROPOSAL_EXECUTION_MASK => {
                Ok(TaggedReplyId::FailedProposalExecution(id_after_shift))
            }
            FAILED_PROPOSAL_HOOK_MASK => Ok(TaggedReplyId::FailedProposalHook(id_after_shift)),
            FAILED_VOTE_HOOK_MASK => Ok(TaggedReplyId::FailedVoteHook(id_after_shift)),
            PRE_PROPOSE_MODULE_INSTANTIATION_MASK => {
                Ok(TaggedReplyId::PreProposeModuleInstantiation)
            }
            _ => Err(error::TagError::UnknownReplyId { id }),
        }
    }
}

/// This function can drop bits, if you have more than `u(64-[`BITS_RESERVED_FOR_REPLY_TYPE`])` proposals.
pub const fn mask_proposal_execution_proposal_id(proposal_id: u64) -> u64 {
    FAILED_PROPOSAL_EXECUTION_MASK | (proposal_id << BITS_RESERVED_FOR_REPLY_TYPE)
}

pub const fn mask_proposal_hook_index(index: u64) -> u64 {
    FAILED_PROPOSAL_HOOK_MASK | (index << BITS_RESERVED_FOR_REPLY_TYPE)
}

pub const fn mask_vote_hook_index(index: u64) -> u64 {
    FAILED_VOTE_HOOK_MASK | (index << BITS_RESERVED_FOR_REPLY_TYPE)
}

pub const fn mask_pre_propose_module_instantiation() -> u64 {
    PRE_PROPOSE_MODULE_INSTANTIATION_MASK
}

pub mod error {
    use thiserror::Error;

    #[derive(Error, Debug, PartialEq, Eq)]
    pub enum TagError {
        #[error("Unknown reply id ({id}).")]
        UnknownReplyId { id: u64 },
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_tagged_reply_id() {
        // max u62, change this if new reply types added
        let proposal_id = 4611686018427387903;
        let proposal_hook_idx = 1234;
        let vote_hook_idx = 4321;

        let m_proposal_id = mask_proposal_execution_proposal_id(proposal_id);
        let m_proposal_hook_idx = mask_proposal_hook_index(proposal_hook_idx);
        let m_vote_hook_idx = mask_vote_hook_index(vote_hook_idx);

        assert_eq!(
            TaggedReplyId::new(m_proposal_id).unwrap(),
            TaggedReplyId::FailedProposalExecution(proposal_id)
        );
        assert_eq!(
            TaggedReplyId::new(m_proposal_hook_idx).unwrap(),
            TaggedReplyId::FailedProposalHook(proposal_hook_idx)
        );
        assert_eq!(
            TaggedReplyId::new(m_vote_hook_idx).unwrap(),
            TaggedReplyId::FailedVoteHook(vote_hook_idx)
        );
    }
}
