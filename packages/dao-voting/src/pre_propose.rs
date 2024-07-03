//! Types related to the pre-propose module. Motivation:
//! <https://github.com/DA0-DA0/dao-contracts/discussions/462>.

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Empty, StdResult, SubMsg};
use dao_interface::state::ModuleInstantiateInfo;
use thiserror::Error;

use crate::reply::pre_propose_module_instantiation_id;

#[cw_serde]
pub enum PreProposeInfo {
    /// Anyone may create a proposal free of charge.
    AnyoneMayPropose {},
    /// The module specified in INFO has exclusive rights to proposal
    /// creation.
    ModuleMayPropose { info: ModuleInstantiateInfo },
}

/// The policy configured in a proposal module that determines whether or not a
/// pre-propose module is in use. If so, only the module can create new
/// proposals. Otherwise, there is no restriction on proposal creation.
#[cw_serde]
pub enum ProposalCreationPolicy {
    /// Anyone may create a proposal, free of charge.
    Anyone {},
    /// Only ADDR may create proposals. It is expected that ADDR is a
    /// pre-propose module, though we only require that it is a valid
    /// address.
    Module { addr: Addr },
}

impl ProposalCreationPolicy {
    /// Determines if CREATOR is permitted to create a
    /// proposal. Returns true if so and false otherwise.
    pub fn is_permitted(&self, creator: &Addr) -> bool {
        match self {
            Self::Anyone {} => true,
            Self::Module { addr } => creator == addr,
        }
    }
}

impl PreProposeInfo {
    pub fn into_initial_policy_and_messages(
        self,
        dao: Addr,
    ) -> StdResult<(ProposalCreationPolicy, Vec<SubMsg<Empty>>)> {
        Ok(match self {
            Self::AnyoneMayPropose {} => (ProposalCreationPolicy::Anyone {}, vec![]),
            Self::ModuleMayPropose { info } => (
                // Anyone can propose will be set until instantiation succeeds, then
                // `ModuleMayPropose` will be set. This ensures that we fail open
                // upon instantiation failure.
                ProposalCreationPolicy::Anyone {},
                vec![SubMsg::reply_on_success(
                    info.into_wasm_msg(dao),
                    pre_propose_module_instantiation_id(),
                )],
            ),
        })
    }
}

/// The policy configured in a pre-propose module that determines who can submit
/// proposals. This is the preferred way to restrict proposal creation (as
/// opposed to the ProposalCreationPolicy above) since pre-propose modules
/// support other features, such as proposal deposits.
#[cw_serde]
pub enum PreProposeSubmissionPolicy {
    /// Anyone may create proposals, except for those in the denylist.
    Anyone {
        /// Addresses that may not create proposals.
        denylist: Option<Vec<String>>,
    },
    /// Specific people may create proposals.
    Specific {
        /// Whether or not DAO members may create proposals.
        dao_members: bool,
        /// Addresses that may create proposals.
        allowlist: Option<Vec<String>>,
        /// Addresses that may not create proposals, overriding other settings.
        denylist: Option<Vec<String>>,
    },
}

#[derive(Error, Debug, PartialEq, Eq)]
pub enum PreProposeSubmissionPolicyError {
    #[error("The proposal submission policy doesn't allow anyone to submit proposals")]
    NoOneAllowed {},

    #[error("Denylist cannot contain addresses in the allowlist")]
    DenylistAllowlistOverlap {},

    #[error("You are not allowed to submit proposals")]
    Unauthorized {},

    #[error("The current proposal submission policy (Anyone) only supports a denylist. Change the policy to Specific in order to configure more granular permissions.")]
    AnyoneInvalidUpdateFields {},
}

impl PreProposeSubmissionPolicy {
    /// Validate the policy configuration.
    pub fn validate(&self) -> Result<(), PreProposeSubmissionPolicyError> {
        if let PreProposeSubmissionPolicy::Specific {
            dao_members,
            allowlist,
            denylist,
        } = self
        {
            let allowlist = allowlist.as_deref().unwrap_or_default();
            let denylist = denylist.as_deref().unwrap_or_default();

            // prevent allowlist and denylist from overlapping
            if denylist.iter().any(|a| allowlist.iter().any(|b| a == b)) {
                return Err(PreProposeSubmissionPolicyError::DenylistAllowlistOverlap {});
            }

            // ensure someone is allowed to submit proposals, be it DAO members
            // or someone on the allowlist. we can't verify that the denylist
            // doesn't contain all DAO members, so this is the best we can do to
            // ensure that someone is allowed to submit.
            if !dao_members && allowlist.is_empty() {
                return Err(PreProposeSubmissionPolicyError::NoOneAllowed {});
            }
        }

        Ok(())
    }

    /// Human readable string for use in events.
    pub fn human_readable(&self) -> String {
        match self {
            Self::Anyone { .. } => "anyone".to_string(),
            Self::Specific { .. } => "specific".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{to_json_binary, WasmMsg};

    use super::*;

    #[test]
    fn test_anyone_is_permitted() {
        let policy = ProposalCreationPolicy::Anyone {};

        // I'll actually stand by this as a legit testing strategy
        // when looking at string inputs. If anything is going to
        // screw things up, its weird unicode characters.
        //
        // For example, my langauge server explodes for me if I use
        // the granddaddy of weird unicode characters, the large
        // family: 👩‍👩‍👧‍👦.
        //
        // The family emoji you see is actually a combination of
        // individual person emojis. You can browse the whole
        // collection of combo emojis here:
        // <https://unicode.org/emoji/charts/emoji-zwj-sequences.html>.
        //
        // You may also enjoy this PDF wherein there is a discussion
        // about the feesability of supporting all 7230 possible
        // combos of family emojis:
        // <https://www.unicode.org/L2/L2020/20114-family-emoji-explor.pdf>.
        for c in '😀'..'🤣' {
            assert!(policy.is_permitted(&Addr::unchecked(c.to_string())))
        }
    }

    #[test]
    fn test_module_is_permitted() {
        let policy = ProposalCreationPolicy::Module {
            addr: Addr::unchecked("deposit_module"),
        };
        assert!(!policy.is_permitted(&Addr::unchecked("👩‍👩‍👧‍👦")));
        assert!(policy.is_permitted(&Addr::unchecked("deposit_module")));
    }

    #[test]
    fn test_pre_any_conversion() {
        let info = PreProposeInfo::AnyoneMayPropose {};
        let (policy, messages) = info
            .into_initial_policy_and_messages(Addr::unchecked("😃"))
            .unwrap();
        assert_eq!(policy, ProposalCreationPolicy::Anyone {});
        assert!(messages.is_empty())
    }

    #[test]
    fn test_pre_module_conversion() {
        let info = PreProposeInfo::ModuleMayPropose {
            info: ModuleInstantiateInfo {
                code_id: 42,
                msg: to_json_binary("foo").unwrap(),
                admin: None,
                funds: vec![],
                label: "pre-propose-9000".to_string(),
            },
        };
        let (policy, messages) = info
            .into_initial_policy_and_messages(Addr::unchecked("🥵"))
            .unwrap();

        // In this case the package is expected to allow anyone to
        // create a proposal (fail-open), and provide some messages
        // that, when handled in a `reply` handler will set the
        // creation policy to a specific module.
        assert_eq!(policy, ProposalCreationPolicy::Anyone {});
        assert_eq!(messages.len(), 1);
        assert_eq!(
            messages[0],
            SubMsg::reply_on_success(
                WasmMsg::Instantiate {
                    admin: None,
                    code_id: 42,
                    msg: to_json_binary("foo").unwrap(),
                    funds: vec![],
                    label: "pre-propose-9000".to_string()
                },
                crate::reply::pre_propose_module_instantiation_id()
            )
        )
    }
}
