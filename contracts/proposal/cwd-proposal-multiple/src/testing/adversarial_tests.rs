use cosmwasm_std::{Addr, CosmosMsg};
use cw_multi_test::App;
use cwd_voting::multiple_choice::{MultipleChoiceOption, MultipleChoiceOptions};
use crate::testing::execute::{make_proposal, mint_cw20s};
use crate::testing::instantiate::{_get_default_token_dao_proposal_module_instantiate, instantiate_with_staked_balances_governance};
use crate::testing::queries::{query_dao_token, query_multiple_proposal_module};
use crate::testing::tests::CREATOR_ADDR;

struct CommonTest {
    app: App,
    core_addr: Addr,
    proposal_module: Addr,
    gov_token: Addr,
    proposal_id: u64,
}
fn setup_test(messages: Vec<CosmosMsg>) -> CommonTest {
    let mut app = App::default();
    let instantiate = _get_default_token_dao_proposal_module_instantiate(&mut app);
    let core_addr = instantiate_with_staked_balances_governance(&mut app, instantiate, None);
    let proposal_module = query_multiple_proposal_module(&app, &core_addr);
    let gov_token = query_dao_token(&app, &core_addr);

    // Mint some tokens to pay the proposal deposit.
    mint_cw20s(&mut app, &gov_token, &core_addr, CREATOR_ADDR, 10_000_000);

    let options = vec![
        MultipleChoiceOption {
            description: "multiple choice option 1".to_string(),
            msgs: None,
        },
        MultipleChoiceOption {
            description: "multiple choice option 2".to_string(),
            msgs: None,
        },
    ];

    let mc_options = MultipleChoiceOptions { options };

    let proposal_id = make_proposal(&mut app, &proposal_module, CREATOR_ADDR, mc_options);

    CommonTest {
        app,
        core_addr,
        proposal_module,
        gov_token,
        proposal_id,
    }
}

#[test]
fn test_execute_proposal_open() {
    let CommonTest {
        mut app,
        core_addr: _,
        proposal_module,
        gov_token: _,
        proposal_id,
    } = setup_test(vec![]);



}

#[test]
fn test_execute_proposal_rejected_closed() {
    unimplemented!()
}

#[test]
fn test_execute_proposal_more_than_once() {
    unimplemented!()
}