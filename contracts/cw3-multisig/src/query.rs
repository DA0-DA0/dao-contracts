use cosmwasm_std::{Addr, CosmosMsg, Decimal, Empty, Uint128};
use cw20::Cw20CoinVerified;
use cw3::Status;
use cw4::Cw4Contract;
use cw_utils::{Expiration, ThresholdResponse};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;

use crate::state::{Config, Votes};

/// Our own custom proposal response class, implements
/// all attributes specified in CW3. Extended as we
/// have a proposer field and want to test if it is set
/// correctly.
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct ProposalResponse<T = Empty>
where
    T: Clone + fmt::Debug + PartialEq + JsonSchema,
{
    pub id: u64,
    pub title: String,
    pub description: String,
    pub proposer: Addr,
    pub msgs: Vec<CosmosMsg<T>>,
    pub status: Status,
    pub expires: Expiration,
    /// This is the threshold that is applied to this proposal. Both the rules of the voting contract,
    /// as well as the total_weight of the voting group may have changed since this time. That means
    /// that the generic `Threshold{}` query does not provide valid information for existing proposals.
    pub threshold: ThresholdResponse,
}

/// As above, implement our own proposal list response
/// each proposal will now have a proposer attached.
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct ProposalListResponse {
    pub proposals: Vec<ProposalResponse>,
}

/// Information about the current status of a proposal.
///
/// NOTE: this response type is not defined in the cw3 spec so we
/// define it ourselves.
/// Information about the current status of a proposal.
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct VoteTallyResponse {
    /// Current proposal status
    pub status: Status,
    /// Required passing criteria
    pub threshold: ThresholdResponse,
    /// Current percentage turnout
    pub quorum: Decimal,
    /// Total number of votes for the proposal
    pub total_votes: Uint128,
    /// Total number of votes possible for the proposal
    pub total_weight: Uint128,
    /// Tally of the different votes
    pub votes: Votes,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct ConfigResponse {
    pub config: Config,
    pub group_address: Cw4Contract,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Cw20BalancesResponse {
    pub cw20_balances: Vec<Cw20CoinVerified>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct TokenListResponse {
    pub token_list: Vec<Addr>,
}
