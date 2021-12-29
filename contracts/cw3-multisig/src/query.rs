use cosmwasm_std::{Addr, Decimal, Uint128};
use cw20::Cw20CoinVerified;
use cw3::Status;
use cw4::Cw4Contract;
use cw_utils::ThresholdResponse;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::state::{Config, Votes};

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
