use cosmwasm_std::{Attribute, Deps};

use crate::{msg::ProposalIncentivesUnchecked, state::ProposalIncentives, ContractError};

impl ProposalIncentivesUnchecked {
    pub fn into_checked(self, deps: Deps) -> Result<ProposalIncentives, ContractError> {
        if self.rewards_per_proposal.is_zero() {
            return Err(ContractError::NoRewardPerProposal {});
        }

        Ok(ProposalIncentives {
            rewards_per_proposal: self.rewards_per_proposal,
            denom: self.denom.into_checked(deps)?,
        })
    }
}

impl ProposalIncentives {
    pub fn into_attributes(&self) -> Vec<Attribute> {
        vec![
            Attribute {
                key: "reward_per_proposal".to_string(),
                value: self.rewards_per_proposal.to_string(),
            },
            Attribute {
                key: "denom".to_string(),
                value: self.denom.to_string(),
            },
        ]
    }
}
