use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{CosmosMsg, Empty};
use dao_pre_propose_base::msg::{
    ExecuteMsg as ExecuteBase, InstantiateMsg as InstantiateBase, QueryMsg as QueryBase,
};

/// The contents of a message to create a proposal.
// We break this type out of `ExecuteMsg` because we want pre-propose
// modules that interact with this contract to be able to get type
// checking on their propose messages.
#[cw_serde]
pub struct ProposeMsg {
    /// The title of the proposal.
    pub title: String,
    /// A description of the proposal.
    pub description: String,
    /// The messages that should be executed in response to this
    /// proposal passing.
    pub msgs: Vec<CosmosMsg<Empty>>,
    /// The address creating the proposal. If no pre-propose
    /// module is attached to this module this must always be None
    /// as the proposer is the sender of the propose message. If a
    /// pre-propose module is attached, this must be Some and will
    /// set the proposer of the proposal it creates.
    pub proposer: Option<String>,
}

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
