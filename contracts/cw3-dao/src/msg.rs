use crate::error::ContractError;
use crate::query::ThresholdResponse;
use crate::state::Config;
use cosmwasm_std::{Addr, CosmosMsg, Decimal, Empty, Uint128};
use cw0::{Duration, Expiration};
use cw20::Cw20Coin;
use cw20_base::msg::InstantiateMarketingInfo;
use cw3::Vote;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct InstantiateMsg {
    // The name of the DAO.
    pub name: String,
    // A description of the DAO.
    pub description: String,
    /// Set an existing governance token or launch a new one
    pub gov_token: GovTokenMsg,
    /// Voting params configuration
    pub threshold: Threshold,
    /// The amount of time a proposal can be voted on before expiring
    pub max_voting_period: Duration,
    /// Deposit required to make a proposal
    pub proposal_deposit_amount: Uint128,
    /// Refund a proposal if it is rejected
    pub refund_failed_proposals: Option<bool>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum GovTokenMsg {
    // Instantiate a new cw20 token with the DAO as minter
    InstantiateNewCw20 {
        cw20_code_id: u64,
        stake_contract_code_id: u64,
        label: String,
        initial_dao_balance: Option<Uint128>,
        msg: GovTokenInstantiateMsg,
        unstaking_duration: Option<Duration>,
    },
    /// Use an existing cw20 token
    UseExistingCw20 {
        addr: String,
        label: String,
        stake_contract_code_id: u64,
        unstaking_duration: Option<Duration>,
    },
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct GovTokenInstantiateMsg {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub initial_balances: Vec<Cw20Coin>,
    pub marketing: Option<InstantiateMarketingInfo>,
}

/// This defines the different ways tallies can happen.
///
/// The total_weight used for calculating success as well as the weights of each
/// individual voter used in tallying should be snapshotted at the beginning of
/// the block at which the proposal starts (this is likely the responsibility of a
/// correct cw4 implementation).
/// See also `ThresholdResponse` in the cw3 spec.
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Threshold {
    /// Declares a percentage of the total weight that must cast Yes votes in order for
    /// a proposal to pass.
    /// See `ThresholdResponse.AbsolutePercentage` in the cw3 spec for details.
    AbsolutePercentage { percentage: Decimal },

    /// Declares a `quorum` of the total votes that must participate in the election in order
    /// for the vote to be considered at all.
    /// See `ThresholdResponse.ThresholdQuorum` in the cw3 spec for details.
    ThresholdQuorum { threshold: Decimal, quorum: Decimal },
}

impl Threshold {
    /// returns error if this is an unreachable value,
    /// given a total weight of all members in the group
    pub fn validate(&self) -> Result<(), ContractError> {
        match self {
            Threshold::AbsolutePercentage {
                percentage: percentage_needed,
            } => valid_percentage(percentage_needed),
            Threshold::ThresholdQuorum {
                threshold,
                quorum: quroum,
            } => {
                valid_percentage(threshold)?;
                valid_percentage(quroum)
            }
        }
    }

    /// Creates a response from the saved data, just missing the total_weight info
    pub fn to_response(&self, total_weight: Uint128) -> ThresholdResponse {
        match self.clone() {
            Threshold::AbsolutePercentage { percentage } => ThresholdResponse::AbsolutePercentage {
                percentage,
                total_weight,
            },
            Threshold::ThresholdQuorum { threshold, quorum } => {
                ThresholdResponse::ThresholdQuorum {
                    threshold,
                    quorum,
                    total_weight,
                }
            }
        }
    }
}

/// Asserts that the 0.0 < percent <= 1.0
fn valid_percentage(percent: &Decimal) -> Result<(), ContractError> {
    if percent.is_zero() {
        Err(ContractError::ZeroThreshold {})
    } else if *percent > Decimal::one() {
        Err(ContractError::UnreachableThreshold {})
    } else {
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ProposeMsg {
    pub title: String,
    pub description: String,
    pub msgs: Vec<CosmosMsg<Empty>>,
    // note: we ignore API-spec'd earliest if passed, always opens immediately
    pub latest: Option<Expiration>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct VoteMsg {
    pub proposal_id: u64,
    pub vote: Vote,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// Makes a new proposal
    Propose(ProposeMsg),
    /// Vote on an open proposal
    Vote(VoteMsg),
    /// Execute a passed proposal
    Execute { proposal_id: u64 },
    /// Close a failed proposal
    Close { proposal_id: u64 },
    /// Update DAO config (can only be called by DAO contract)
    UpdateConfig(Config),
    /// Updates token list
    UpdateCw20TokenList {
        to_add: Vec<Addr>,
        to_remove: Vec<Addr>,
    },
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Return ThresholdResponse
    Threshold {},
    /// Returns ProposalResponse
    Proposal { proposal_id: u64 },
    /// Returns ProposalListResponse
    ListProposals {
        start_after: Option<u64>,
        limit: Option<u32>,
    },
    /// Returns ProposalListResponseb
    ReverseProposals {
        start_before: Option<u64>,
        limit: Option<u32>,
    },
    /// Returns the number of proposals in the DAO (u64)
    ProposalCount {},
    /// Returns VoteResponse
    Vote { proposal_id: u64, voter: String },
    /// Returns VoteListResponse
    ListVotes {
        proposal_id: u64,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// Returns information about current tallys for a
    /// proposal. Returns type `VoteTallyResponse`.
    Tally { proposal_id: u64 },
    /// Returns VoterInfo
    Voter { address: String },
    /// Returns Config
    GetConfig {},
    /// Returns All DAO Cw20 Balances
    Cw20Balances {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// Return list of cw20 Tokens associated with the DAO Treasury
    Cw20TokenList {},
}

#[cfg(test)]
mod tests {
    use super::*;

    use cosmwasm_std::to_vec;

    #[test]
    fn vote_encoding() {
        let a = Vote::Yes;
        let encoded = to_vec(&a).unwrap();
        let json = String::from_utf8_lossy(&encoded).to_string();
        assert_eq!(r#""yes""#, json.as_str());
    }

    #[test]
    fn vote_encoding_embedded() {
        let msg = ExecuteMsg::Vote(VoteMsg {
            proposal_id: 17,
            vote: Vote::No,
        });
        let encoded = to_vec(&msg).unwrap();
        let json = String::from_utf8_lossy(&encoded).to_string();
        assert_eq!(r#"{"vote":{"proposal_id":17,"vote":"no"}}"#, json.as_str());
    }

    #[test]
    fn validate_percentage() {
        // 0 is never a valid percentage
        let err = valid_percentage(&Decimal::zero()).unwrap_err();
        assert_eq!(err.to_string(), ContractError::ZeroThreshold {}.to_string());

        // 100% is
        valid_percentage(&Decimal::one()).unwrap();

        // 101% is not
        let err = valid_percentage(&Decimal::percent(101)).unwrap_err();
        assert_eq!(
            err.to_string(),
            ContractError::UnreachableThreshold {}.to_string()
        );
        // not 100.1%
        let err = valid_percentage(&Decimal::permille(1001)).unwrap_err();
        assert_eq!(
            err.to_string(),
            ContractError::UnreachableThreshold {}.to_string()
        );

        // other values in between 0 and 1 are valid
        valid_percentage(&Decimal::permille(1)).unwrap();
        valid_percentage(&Decimal::percent(17)).unwrap();
        valid_percentage(&Decimal::percent(99)).unwrap();
    }

    #[test]
    fn validate_threshold() {
        // AbsolutePercentage just enforces valid_percentage (tested above)
        let err = Threshold::AbsolutePercentage {
            percentage: Decimal::zero(),
        }
        .validate()
        .unwrap_err();
        assert_eq!(err.to_string(), ContractError::ZeroThreshold {}.to_string());
        Threshold::AbsolutePercentage {
            percentage: Decimal::percent(51),
        }
        .validate()
        .unwrap();

        // Quorum enforces both valid just enforces valid_percentage (tested above)
        Threshold::ThresholdQuorum {
            threshold: Decimal::percent(51),
            quorum: Decimal::percent(40),
        }
        .validate()
        .unwrap();
        let err = Threshold::ThresholdQuorum {
            threshold: Decimal::percent(101),
            quorum: Decimal::percent(40),
        }
        .validate()
        .unwrap_err();
        assert_eq!(
            err.to_string(),
            ContractError::UnreachableThreshold {}.to_string()
        );
        let err = Threshold::ThresholdQuorum {
            threshold: Decimal::percent(51),
            quorum: Decimal::percent(0),
        }
        .validate()
        .unwrap_err();
        assert_eq!(err.to_string(), ContractError::ZeroThreshold {}.to_string());
    }

    #[test]
    fn threshold_response() {
        let total_weight = Uint128::new(100);

        let res = Threshold::AbsolutePercentage {
            percentage: Decimal::percent(51),
        }
        .to_response(total_weight);
        assert_eq!(
            res,
            ThresholdResponse::AbsolutePercentage {
                percentage: Decimal::percent(51),
                total_weight
            }
        );

        let res = Threshold::ThresholdQuorum {
            threshold: Decimal::percent(66),
            quorum: Decimal::percent(50),
        }
        .to_response(total_weight);
        assert_eq!(
            res,
            ThresholdResponse::ThresholdQuorum {
                threshold: Decimal::percent(66),
                quorum: Decimal::percent(50),
                total_weight
            }
        );
    }
}
