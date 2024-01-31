use cosmwasm_std::{Deps, StdResult};

use crate::state::{ProposalIncentives, PROPOSAL_INCENTIVES};

pub fn proposal_incentives(deps: Deps, height: Option<u64>) -> StdResult<ProposalIncentives> {
    match height {
        Some(height) => PROPOSAL_INCENTIVES
            .may_load_at_height(deps.storage, height)?
            .ok_or(cosmwasm_std::StdError::NotFound {
                kind: "Proposal Incentives".to_string(),
            }),
        None => PROPOSAL_INCENTIVES.load(deps.storage),
    }
}
