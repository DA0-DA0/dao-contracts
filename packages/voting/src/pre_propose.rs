//! Types related to the pre-propose module. Motivation:
//! <https://github.com/DA0-DA0/dao-contracts/discussions/462>.

use cosmwasm_std::{Addr, Deps, Empty, StdResult, SubMsg};
use cw_core_interface::ModuleInstantiateInfo;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::reply::mask_pre_propose_module_instantiation;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub enum PreProposeInfo {
    /// Anyone may create a proposal free of charge.
    AnyoneMayPropose {},
    /// The module specified in INFO has exclusive rights to proposal
    /// creation.
    ModuleMayPropose { info: ModuleInstantiateInfo },
    /// The address specified in ADDR has exclusive rights to proposal
    /// creation.
    AddrMayPropose { addr: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
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

    /// Determines if ADDR is the module registered to create
    /// proposals by this proposal creation policy. Returns true if so
    /// and false otherwise.
    pub fn addr_is_my_module(&self, addr: &Addr) -> bool {
        match self {
            Self::Anyone {} => false,
            ProposalCreationPolicy::Module { addr: module_addr } => module_addr == addr,
        }
    }
}

impl PreProposeInfo {
    pub fn into_initial_policy_and_messages(
        self,
        contract_address: Addr,
        deps: Deps,
    ) -> StdResult<(ProposalCreationPolicy, Vec<SubMsg<Empty>>)> {
        Ok(match self {
            Self::AnyoneMayPropose {} => (ProposalCreationPolicy::Anyone {}, vec![]),
            Self::AddrMayPropose { addr } => (
                ProposalCreationPolicy::Module {
                    addr: deps.api.addr_validate(&addr)?,
                },
                vec![],
            ),
            Self::ModuleMayPropose { info } => (
                ProposalCreationPolicy::Anyone {},
                // If the instantiation of the pre-propose module fails,
                // we fail the entire instantiation. This is in contrast
                // to the normal fail-open behavior of this module.
                vec![SubMsg::reply_on_success(
                    info.into_wasm_msg(contract_address),
                    mask_pre_propose_module_instantiation(),
                )],
            ),
        })
    }
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{testing::mock_dependencies, to_binary, WasmMsg};

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
        let deps = mock_dependencies();
        let (policy, messages) = info
            .into_initial_policy_and_messages(Addr::unchecked("😃"), deps.as_ref())
            .unwrap();
        assert_eq!(policy, ProposalCreationPolicy::Anyone {});
        assert!(messages.is_empty())
    }

    #[test]
    fn test_pre_addr_conversion() {
        let info = PreProposeInfo::AddrMayPropose {
            addr: "😅".to_string(),
        };
        let deps = mock_dependencies();
        let (policy, messages) = info
            .into_initial_policy_and_messages(Addr::unchecked("🥵"), deps.as_ref())
            .unwrap();
        assert_eq!(
            policy,
            ProposalCreationPolicy::Module {
                addr: Addr::unchecked("😅")
            }
        );
        assert!(messages.is_empty())
    }

    #[test]
    fn test_pre_module_conversion() {
        let info = PreProposeInfo::ModuleMayPropose {
            info: ModuleInstantiateInfo {
                code_id: 42,
                msg: to_binary("foo").unwrap(),
                admin: None,
                label: "pre-propose-9000".to_string(),
            },
        };
        let deps = mock_dependencies();
        let (policy, messages) = info
            .into_initial_policy_and_messages(Addr::unchecked("🥵"), deps.as_ref())
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
                    msg: to_binary("foo").unwrap(),
                    funds: vec![],
                    label: "pre-propose-9000".to_string()
                },
                crate::reply::mask_pre_propose_module_instantiation()
            )
        )
    }
}
