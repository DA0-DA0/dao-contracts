use cosmwasm_std::{Addr, CosmosMsg};
use cw_multi_test::{next_block, App};

use crate::testing::{
    execute::{
        close_proposal, execute_proposal, execute_proposal_should_fail, make_proposal, mint_cw20s,
        vote_on_proposal,
    },
    instantiate::{
        get_default_token_dao_proposal_module_instantiate,
        instantiate_with_staked_balances_governance,
    },
    queries::{query_dao_token, query_proposal, query_single_proposal_module},
};
use cwd_voting::{status::Status, voting::Vote};

use super::CREATOR_ADDR;
use crate::{query::ProposalResponse, ContractError};

struct CommonTest {
    app: App,
    proposal_module: Addr,
    proposal_id: u64,
}
fn setup_test(messages: Vec<CosmosMsg>) -> CommonTest {
    let mut app = App::default();
    let instantiate = get_default_token_dao_proposal_module_instantiate(&mut app);
    let core_addr = instantiate_with_staked_balances_governance(&mut app, instantiate, None);
    let proposal_module = query_single_proposal_module(&app, &core_addr);
    let gov_token = query_dao_token(&app, &core_addr);

    // Mint some tokens to pay the proposal deposit.
    mint_cw20s(&mut app, &gov_token, &core_addr, CREATOR_ADDR, 10_000_000);
    let proposal_id = make_proposal(&mut app, &proposal_module, CREATOR_ADDR, messages);

    CommonTest {
        app,
        proposal_module,
        proposal_id,
    }
}

// A proposal that is still accepting votes (is open) cannot
// be executed. Any attempts to do so should fail and return
// an error.
#[test]
fn test_execute_proposal_open() {
    let CommonTest {
        mut app,
        proposal_module,
        proposal_id,
    } = setup_test(vec![]);

    app.update_block(next_block);

    // assert proposal is open
    let proposal = query_proposal(&app, &proposal_module, proposal_id);
    assert_eq!(proposal.proposal.status, Status::Open);

    // attempt to execute and assert that it fails
    let err = execute_proposal_should_fail(&mut app, &proposal_module, CREATOR_ADDR, proposal_id);
    assert!(matches!(err, ContractError::NotPassed {}))
}

// A proposal can be executed if and only if it passed.
// Any attempts to execute a proposal that has been rejected
// or closed (after rejection) should fail and return an error.
#[test]
fn test_execute_proposal_rejected_closed() {
    let CommonTest {
        mut app,
        proposal_module,
        proposal_id,
    } = setup_test(vec![]);

    // Assert proposal is open and vote enough to reject it
    let proposal: ProposalResponse = query_proposal(&app, &proposal_module, 1);
    assert_eq!(proposal.proposal.status, Status::Open);
    vote_on_proposal(
        &mut app,
        &proposal_module,
        CREATOR_ADDR,
        proposal_id,
        Vote::No,
    );

    app.update_block(next_block);

    // Assert proposal is rejected
    let proposal: ProposalResponse = query_proposal(&app, &proposal_module, proposal_id);
    assert_eq!(proposal.proposal.status, Status::Rejected);

    // Attempt to execute
    let err = execute_proposal_should_fail(&mut app, &proposal_module, CREATOR_ADDR, proposal_id);
    assert!(matches!(err, ContractError::NotPassed {}));

    app.update_block(next_block);

    // close the proposal
    close_proposal(&mut app, &proposal_module, CREATOR_ADDR, proposal_id);
    let proposal = query_proposal(&app, &proposal_module, proposal_id);
    assert_eq!(proposal.proposal.status, Status::Closed);

    // Attempt to execute
    let err = execute_proposal_should_fail(&mut app, &proposal_module, CREATOR_ADDR, proposal_id);
    assert!(matches!(err, ContractError::NotPassed {}))
}

// A proposal can only be executed once. Any subsequent
// attempts to execute it should fail and return an error.
#[test]
fn test_execute_proposal_more_than_once() {
    let CommonTest {
        mut app,
        proposal_module,
        proposal_id,
    } = setup_test(vec![]);

    // Assert proposal is open and vote enough to reject it
    let proposal: ProposalResponse = query_proposal(&app, &proposal_module, proposal_id);
    assert_eq!(proposal.proposal.status, Status::Open);
    vote_on_proposal(
        &mut app,
        &proposal_module,
        CREATOR_ADDR,
        proposal_id,
        Vote::Yes,
    );

    app.update_block(next_block);

    // assert proposal is passed, execute it
    let proposal = query_proposal(&app, &proposal_module, proposal_id);
    assert_eq!(proposal.proposal.status, Status::Passed);
    execute_proposal(&mut app, &proposal_module, CREATOR_ADDR, proposal_id);

    app.update_block(next_block);

    // assert proposal executed and attempt to execute it again
    let proposal = query_proposal(&app, &proposal_module, proposal_id);
    assert_eq!(proposal.proposal.status, Status::Executed);
    let err: ContractError =
        execute_proposal_should_fail(&mut app, &proposal_module, CREATOR_ADDR, proposal_id);
    assert!(matches!(err, ContractError::NotPassed {}));
}
