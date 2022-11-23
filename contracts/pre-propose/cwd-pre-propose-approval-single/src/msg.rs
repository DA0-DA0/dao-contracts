use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{CosmosMsg, Empty};
use cwd_pre_propose_base::msg::{
    ExecuteMsg as ExecuteBase, InstantiateMsg as InstantiateBase, QueryMsg as QueryBase,
};
use cwd_proposal_single::msg::ProposeMsg;

#[cw_serde]
pub enum ApproverProposeMessage {
    Propose {
        title: String,
        description: String,
        approval_id: u64,
    },
}

#[cw_serde]
pub enum ProposeMessage {
    Propose {
        title: String,
        description: String,
        msgs: Vec<CosmosMsg<Empty>>,
    },
}

#[cw_serde]
pub struct InstantiateExt {
    pub approver: String,
}

#[cw_serde]
pub enum ExecuteExt {
    /// Approve a proposal, only callable by approver
    Approve { id: u64 },
    /// Reject a proposal, only callable by approver
    Reject { id: u64 },
    /// Updates the approver, can only be called the current approver
    UpdateApprover { address: String },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryExt {
    /// List the approver address
    #[returns(cosmwasm_std::Addr)]
    Approver {},
    /// A pending proposal
    #[returns(crate::state::PendingProposal)]
    PendingProposal { id: u64 },
    /// List of proposals awaiting approval
    #[returns(Vec<crate::state::PendingProposal>)]
    PendingProposals {
        start_after: Option<u64>,
        limit: Option<u32>,
    },
    #[returns(Vec<crate::state::PendingProposal>)]
    ReversePendingProposals {
        start_before: Option<u64>,
        limit: Option<u32>,
    },
}

pub type InstantiateMsg = InstantiateBase<InstantiateExt>;
pub type ExecuteMsg = ExecuteBase<ProposeMessage, ExecuteExt>;
pub type QueryMsg = QueryBase<QueryExt>;

/// Internal version of the propose message that includes the
/// `proposer` field. The module will fill this in based on the sender
/// of the external message.
#[cw_serde]
pub(crate) enum ProposeMessageInternal {
    Propose(ProposeMsg),
}
