use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{CosmosMsg, Empty};

use cw_utils::Expiration;

use crate::state::Delegation;

#[cw_serde]
pub struct InstantiateMsg {
    pub admin: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    Delegate {
        delegate: String,
        msgs: Vec<CosmosMsg<Empty>>,
        expiration: Option<Expiration>,

        policy_irrevocable: Option<bool>,
        policy_preserve_on_failure: Option<bool>,
    },
    /// Fails if delegation is non-revocable
    RemoveDelegation { delegation_id: u64 },
    /// Only delegate can execute
    Execute { delegation_id: u64 },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Delegation)]
    Delegation { delegation_id: u64 },
    #[returns(Vec<Delegation>)]
    Delegations {
        start_after: Option<u64>,
        limit: Option<u32>,
    },
}

pub type DelegationResponse = Delegation;
