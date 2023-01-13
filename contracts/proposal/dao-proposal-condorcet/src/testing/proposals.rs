use cosmwasm_std::{coins, BankMsg, CosmosMsg};

use crate::{proposal::ProposalResponse, tally::Winner, ContractError};

use super::{is_error, suite::SuiteBuilder};

fn unimportant_message() -> CosmosMsg {
    BankMsg::Send {
        to_address: "someone".to_string(),
        amount: coins(10, "something"),
    }
    .into()
}

#[test]
fn test_make_proposal() {
    let mut suite = SuiteBuilder::default().build();
    let id = suite
        .propose(suite.sender.clone(), vec![vec![unimportant_message()]])
        .unwrap();
    let ProposalResponse { proposal, tally } = suite.query_proposal(id);

    assert_eq!(proposal.id, id);
    assert_eq!(proposal.choices.len(), 2);
    assert_eq!(proposal.choices[0].msgs[0], unimportant_message());

    assert_eq!(tally.candidates(), 2);
    assert_eq!(tally.winner, Winner::None);
    assert_eq!(tally.power_outstanding, proposal.total_power);
    assert_eq!(tally.start_height, suite.block_height());
}

#[test]
fn test_proposal_zero_choices() {
    let mut suite = SuiteBuilder::default().build();
    let err = suite.propose(suite.sender.clone(), vec![]);
    is_error!(err, &ContractError::ZeroChoices {}.to_string());
}

#[test]
fn test_proposal_zero_voting_power() {
    let mut suite = SuiteBuilder::default().build();
    let err = suite.propose("someone", vec![]);
    is_error!(err, &ContractError::ZeroVotingPower {}.to_string());
}
