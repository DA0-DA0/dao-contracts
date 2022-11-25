use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{CosmosMsg, Empty};
use dao_pre_propose_approval_single::msg::ApproverProposeMessage;
use dao_pre_propose_base::msg::{
    ExecuteMsg as ExecuteBase, InstantiateMsg as InstantiateBase, QueryMsg as QueryBase,
};

#[cw_serde]
pub struct InstantiateMsg {
    pub pre_propose_approval_contract: String,
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryExt {
    #[returns(cosmwasm_std::Addr)]
    PreProposeApprovalContract {},
}

pub type BaseInstantiateMsg = InstantiateBase<Empty>;
pub type ExecuteMsg = ExecuteBase<ApproverProposeMessage, Empty>;
pub type QueryMsg = QueryBase<QueryExt>;

/// Internal version of the propose message that includes the
/// `proposer` field. The module will fill this in based on the sender
/// of the external message.
#[cw_serde]
pub enum ProposeMessageInternal {
    Propose {
        title: String,
        description: String,
        msgs: Vec<CosmosMsg<Empty>>,
        proposer: Option<String>,
    },
}
