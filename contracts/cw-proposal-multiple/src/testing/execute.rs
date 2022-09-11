use cosmwasm_std::{coins, Addr, Coin, CosmosMsg, Uint128};
use cw_multi_test::{App, BankSudo, Executor};

use cw_denom::CheckedDenom;
use cw_pre_propose_base_proposal_multiple as cppbpm;
use testing::contracts::cw20_contract;
use voting::{deposit::CheckedDepositInfo, pre_propose::ProposalCreationPolicy, voting::Vote};

use crate::{
    msg::{ExecuteMsg, QueryMsg},
    query::ProposalResponse,
    state::{MultipleChoiceOption, MultipleChoiceOptions},
    testing::{
        queries::{query_pre_proposal_multiple_config, query_proposal_config},
        tests::CREATOR_ADDR,
    },
    ContractError,
};

impl From<MultipleChoiceOptions> for cw_proposal_multiple::state::MultipleChoiceOptions {
    fn from(choices: MultipleChoiceOptions) -> cw_proposal_multiple::state::MultipleChoiceOptions {
        cw_proposal_multiple::state::MultipleChoiceOptions {
            options: choices
                .options
                .into_iter()
                .map(|option| cw_proposal_multiple::state::MultipleChoiceOption {
                    description: option.description,
                    msgs: option.msgs,
                })
                .collect(),
        }
    }
}

impl From<cw_proposal_multiple::state::MultipleChoiceOptions> for MultipleChoiceOptions {
    fn from(choices: cw_proposal_multiple::state::MultipleChoiceOptions) -> MultipleChoiceOptions {
        MultipleChoiceOptions {
            options: choices
                .options
                .into_iter()
                .map(|option| MultipleChoiceOption {
                    description: option.description,
                    msgs: option.msgs,
                })
                .collect(),
        }
    }
}

// Creates a proposal then checks that the proposal was created with
// the specified messages and returns the ID of the proposal.
//
// This expects that the proposer already has the needed tokens to pay
// the deposit.
pub fn make_proposal(
    app: &mut App,
    proposal_multiple: &Addr,
    proposer: &str,
    choices: MultipleChoiceOptions,
) -> u64 {
    let config = query_proposal_config(app, proposal_multiple);

    // Collect the funding.
    let funds = match config.proposal_creation_policy {
        ProposalCreationPolicy::Anyone {} => vec![],
        ProposalCreationPolicy::Module {
            addr: ref pre_propose,
        } => {
            let deposit_config = query_pre_proposal_multiple_config(app, pre_propose);
            match deposit_config.deposit_info {
                Some(CheckedDepositInfo {
                    denom,
                    amount,
                    refund_policy: _,
                }) => match denom {
                    CheckedDenom::Native(denom) => coins(amount.u128(), denom),
                    CheckedDenom::Cw20(addr) => {
                        // Give an allowance, no funds.
                        app.execute_contract(
                            Addr::unchecked(proposer),
                            addr,
                            &cw20::Cw20ExecuteMsg::IncreaseAllowance {
                                spender: pre_propose.to_string(),
                                amount,
                                expires: None,
                            },
                            &[],
                        )
                        .unwrap();
                        vec![]
                    }
                },
                None => vec![],
            }
        }
    };

    // Make the proposal.
    let res = match config.proposal_creation_policy {
        ProposalCreationPolicy::Anyone {} => app
            .execute_contract(
                Addr::unchecked(proposer),
                proposal_multiple.clone(),
                &ExecuteMsg::Propose {
                    title: "title".to_string(),
                    description: "description".to_string(),
                    choices: choices.clone(),
                    proposer: None,
                },
                &[],
            )
            .unwrap(),
        ProposalCreationPolicy::Module { addr } => app
            .execute_contract(
                Addr::unchecked(proposer),
                addr,
                &cppbpm::ExecuteMsg::Propose {
                    msg: cppbpm::ProposeMessage::Propose {
                        title: "title".to_string(),
                        description: "description".to_string(),
                        choices: choices.clone().into(),
                    },
                },
                &funds,
            )
            .unwrap(),
    };

    // The new proposal hook is the last message that fires in
    // this process so we get the proposal ID from it's
    // attributes. We could do this by looking at the proposal
    // creation attributes but this changes relative position
    // depending on if a cw20 or native deposit is being used.
    let attrs = res.custom_attrs(res.events.len() - 1);
    let id = attrs[attrs.len() - 1]
        .value
        .parse()
        // If the proposal creation policy doesn't involve a
        // pre-propose module, no hook so we do it manaually.
        .unwrap_or_else(|_| res.custom_attrs(1)[2].value.parse().unwrap());

    // Check that the proposal was created as expected.
    let proposal: ProposalResponse = app
        .wrap()
        .query_wasm_smart(proposal_multiple, &QueryMsg::Proposal { proposal_id: id })
        .unwrap();

    assert_eq!(proposal.proposal.proposer, Addr::unchecked(proposer));
    assert_eq!(proposal.proposal.title, "title".to_string());
    assert_eq!(proposal.proposal.description, "description".to_string());

    id
}
