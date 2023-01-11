use cosmwasm_schema::cw_serde;
use cosmwasm_std::{BlockInfo, CosmosMsg, Uint128};
use cw_utils::Expiration;
use dao_voting::{status::Status, threshold::PercentageThreshold, voting::does_vote_count_pass};

use crate::tally::{Tally, Winner};

#[cw_serde]
pub struct Proposal {
    last_status: Status,

    pub id: u32,
    pub msgs: Vec<CosmosMsg>,

    pub quorum: PercentageThreshold,
    pub expiration: Expiration,

    pub start_height: u64,
    pub total_power: Uint128,
}

// there also exists some unchecked proposal type that is passed in
// with ExecuteMsg::Propose. fields like total_power can be filled in
// during the transformation to checked form.

pub fn status(block: &BlockInfo, proposal: &Proposal, tally: &Tally) -> Status {
    match proposal.last_status {
        Status::Rejected
        | Status::Passed
        | Status::Executed
        | Status::Closed
        | Status::ExecutionFailed => proposal.last_status,
        Status::Open => {
            let winner = tally.winner;
            let expired = proposal.expiration.is_expired(block);
            let quorum = does_vote_count_pass(
                proposal.total_power - tally.power_outstanding,
                proposal.total_power,
                proposal.quorum,
            );

            if expired && !quorum {
                Status::Rejected
            } else {
                match winner {
                    Winner::Never => Status::Rejected,
                    Winner::None => {
                        if expired {
                            Status::Rejected
                        } else {
                            Status::Open
                        }
                    }
                    Winner::Some(_) => {
                        if expired && quorum {
                            Status::Passed
                        } else {
                            Status::Open
                        }
                    }
                    Winner::Undisputed(_) => {
                        if quorum {
                            Status::Passed
                        } else {
                            Status::Open
                        }
                    }
                }
            }
        }
    }
}
