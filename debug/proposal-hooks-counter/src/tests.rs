use cosmwasm_std::{to_binary, Addr, Empty, Uint128};
use cw20::Cw20Coin;
use cw_multi_test::{App, Contract, ContractWrapper, Executor};
use indexable_hooks::HooksResponse;

use voting::{
    threshold::{PercentageThreshold, Threshold},
    voting::Vote,
};

use crate::msg::{CountResponse, InstantiateMsg, QueryMsg};
use cw_proposal_single::state::Config;

const CREATOR_ADDR: &str = "creator";

fn cw20_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cw20_base::contract::execute,
        cw20_base::contract::instantiate,
        cw20_base::contract::query,
    );
    Box::new(contract)
}

fn single_govmod_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cw_proposal_single::contract::execute,
        cw_proposal_single::contract::instantiate,
        cw_proposal_single::contract::query,
    )
    .with_reply(cw_proposal_single::contract::reply);
    Box::new(contract)
}

fn cw20_balances_voting() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cw20_balance_voting::contract::execute,
        cw20_balance_voting::contract::instantiate,
        cw20_balance_voting::contract::query,
    )
    .with_reply(cw20_balance_voting::contract::reply);
    Box::new(contract)
}

fn cw_gov_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cw_core::contract::execute,
        cw_core::contract::instantiate,
        cw_core::contract::query,
    )
    .with_reply(cw_core::contract::reply);
    Box::new(contract)
}

fn counters_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    );
    Box::new(contract)
}

fn instantiate_governance(app: &mut App, code_id: u64, msg: cw_core::msg::InstantiateMsg) -> Addr {
    app.instantiate_contract(
        code_id,
        Addr::unchecked(CREATOR_ADDR),
        &msg,
        &[],
        "cw-governance",
        None,
    )
    .unwrap()
}

fn instantiate_with_default_governance(
    app: &mut App,
    code_id: u64,
    msg: cw_proposal_single::msg::InstantiateMsg,
    initial_balances: Option<Vec<Cw20Coin>>,
) -> Addr {
    let cw20_id = app.store_code(cw20_contract());
    let governance_id = app.store_code(cw_gov_contract());
    let votemod_id = app.store_code(cw20_balances_voting());

    let initial_balances = initial_balances.unwrap_or_else(|| {
        vec![Cw20Coin {
            address: CREATOR_ADDR.to_string(),
            amount: Uint128::new(100_000_000),
        }]
    });

    let governance_instantiate = cw_core::msg::InstantiateMsg {
        admin: None,
        name: "DAO DAO".to_string(),
        description: "A DAO that builds DAOs".to_string(),
        image_url: None,
        automatically_add_cw20s: true,
        automatically_add_cw721s: true,
        voting_module_instantiate_info: cw_core::msg::ModuleInstantiateInfo {
            code_id: votemod_id,
            msg: to_binary(&cw20_balance_voting::msg::InstantiateMsg {
                token_info: cw20_balance_voting::msg::TokenInfo::New {
                    code_id: cw20_id,
                    label: "DAO DAO governance token".to_string(),
                    name: "DAO".to_string(),
                    symbol: "DAO".to_string(),
                    decimals: 6,
                    initial_balances,
                    marketing: None,
                },
            })
            .unwrap(),
            admin: cw_core::msg::Admin::CoreContract {},
            label: "DAO DAO voting module".to_string(),
        },
        proposal_modules_instantiate_info: vec![cw_core::msg::ModuleInstantiateInfo {
            code_id,
            msg: to_binary(&msg).unwrap(),
            admin: cw_core::msg::Admin::CoreContract {},
            label: "DAO DAO governance module".to_string(),
        }],
        initial_items: None,
    };

    instantiate_governance(app, governance_id, governance_instantiate)
}

#[test]
fn test_counters() {
    let mut app = App::default();
    let govmod_id = app.store_code(single_govmod_contract());
    let counters_id = app.store_code(counters_contract());

    let threshold = Threshold::AbsolutePercentage {
        percentage: PercentageThreshold::Majority {},
    };
    let max_voting_period = cw_utils::Duration::Height(6);
    let instantiate = cw_proposal_single::msg::InstantiateMsg {
        threshold,
        max_voting_period,
        min_voting_period: None,
        only_members_execute: false,
        allow_revoting: false,
        deposit_info: None,
    };

    let governance_addr =
        instantiate_with_default_governance(&mut app, govmod_id, instantiate, None);
    let governance_modules: Vec<Addr> = app
        .wrap()
        .query_wasm_smart(
            governance_addr,
            &cw_core::msg::QueryMsg::ProposalModules {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(governance_modules.len(), 1);
    let govmod_single = governance_modules.into_iter().next().unwrap();

    let govmod_config: Config = app
        .wrap()
        .query_wasm_smart(
            govmod_single.clone(),
            &cw_proposal_single::msg::QueryMsg::Config {},
        )
        .unwrap();
    let dao = govmod_config.dao;

    let counters: Addr = app
        .instantiate_contract(
            counters_id,
            Addr::unchecked(CREATOR_ADDR),
            &InstantiateMsg {
                should_error: false,
            },
            &[],
            "counters",
            None,
        )
        .unwrap();
    let failing_counters: Addr = app
        .instantiate_contract(
            counters_id,
            Addr::unchecked(CREATOR_ADDR),
            &InstantiateMsg { should_error: true },
            &[],
            "failing counters",
            None,
        )
        .unwrap();

    // Register both hooks
    app.execute_contract(
        dao.clone(),
        govmod_single.clone(),
        &cw_proposal_single::msg::ExecuteMsg::AddProposalHook {
            address: counters.to_string(),
        },
        &[],
    )
    .unwrap();
    app.execute_contract(
        dao.clone(),
        govmod_single.clone(),
        &cw_proposal_single::msg::ExecuteMsg::AddVoteHook {
            address: counters.to_string(),
        },
        &[],
    )
    .unwrap();

    // Query both hooks
    let hooks: HooksResponse = app
        .wrap()
        .query_wasm_smart(
            govmod_single.clone(),
            &cw_proposal_single::msg::QueryMsg::ProposalHooks {},
        )
        .unwrap();
    assert_eq!(hooks.hooks.len(), 1);
    let hooks: HooksResponse = app
        .wrap()
        .query_wasm_smart(
            govmod_single.clone(),
            &cw_proposal_single::msg::QueryMsg::VoteHooks {},
        )
        .unwrap();
    assert_eq!(hooks.hooks.len(), 1);

    // Query proposal counter, expect 0
    let resp: CountResponse = app
        .wrap()
        .query_wasm_smart(counters.clone(), &QueryMsg::ProposalCounter {})
        .unwrap();
    assert_eq!(resp.count, 0);

    // Create a new proposal.
    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        govmod_single.clone(),
        &cw_proposal_single::msg::ExecuteMsg::Propose {
            title: "A simple text proposal".to_string(),
            description: "This is a simple text proposal".to_string(),
            msgs: vec![],
        },
        &[],
    )
    .unwrap();

    // Query proposal counter, expect 1
    let resp: CountResponse = app
        .wrap()
        .query_wasm_smart(counters.clone(), &QueryMsg::ProposalCounter {})
        .unwrap();
    assert_eq!(resp.count, 1);

    // Query vote counter, expect 0
    let resp: CountResponse = app
        .wrap()
        .query_wasm_smart(counters.clone(), &QueryMsg::VoteCounter {})
        .unwrap();
    assert_eq!(resp.count, 0);

    // Query status changed counter, expect 0
    let resp: CountResponse = app
        .wrap()
        .query_wasm_smart(counters.clone(), &QueryMsg::StatusChangedCounter {})
        .unwrap();
    assert_eq!(resp.count, 0);

    // Vote
    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        govmod_single.clone(),
        &cw_proposal_single::msg::ExecuteMsg::Vote {
            proposal_id: 1,
            vote: Vote::Yes,
        },
        &[],
    )
    .unwrap();

    // Query vote counter, expect 1
    let resp: CountResponse = app
        .wrap()
        .query_wasm_smart(counters.clone(), &QueryMsg::VoteCounter {})
        .unwrap();
    assert_eq!(resp.count, 1);

    // Query status changed counter, expect 1
    let resp: CountResponse = app
        .wrap()
        .query_wasm_smart(counters.clone(), &QueryMsg::StatusChangedCounter {})
        .unwrap();
    assert_eq!(resp.count, 1);

    // Register the failing hooks
    app.execute_contract(
        dao.clone(),
        govmod_single.clone(),
        &cw_proposal_single::msg::ExecuteMsg::AddProposalHook {
            address: failing_counters.to_string(),
        },
        &[],
    )
    .unwrap();
    app.execute_contract(
        dao.clone(),
        govmod_single.clone(),
        &cw_proposal_single::msg::ExecuteMsg::AddVoteHook {
            address: failing_counters.to_string(),
        },
        &[],
    )
    .unwrap();

    // Expect 2 for each hook
    let hooks: HooksResponse = app
        .wrap()
        .query_wasm_smart(
            govmod_single.clone(),
            &cw_proposal_single::msg::QueryMsg::ProposalHooks {},
        )
        .unwrap();
    assert_eq!(hooks.hooks.len(), 2);
    let hooks: HooksResponse = app
        .wrap()
        .query_wasm_smart(
            govmod_single.clone(),
            &cw_proposal_single::msg::QueryMsg::VoteHooks {},
        )
        .unwrap();
    assert_eq!(hooks.hooks.len(), 2);

    // Create a new proposal.
    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        govmod_single.clone(),
        &cw_proposal_single::msg::ExecuteMsg::Propose {
            title: "A simple text proposal 2nd".to_string(),
            description: "This is a simple text proposal 2nd".to_string(),
            msgs: vec![],
        },
        &[],
    )
    .unwrap();

    // The success counters should still work
    // Query proposal counter, expect 2
    let resp: CountResponse = app
        .wrap()
        .query_wasm_smart(counters.clone(), &QueryMsg::ProposalCounter {})
        .unwrap();
    assert_eq!(resp.count, 2);

    // The contract should of removed the failing counters
    let hooks: HooksResponse = app
        .wrap()
        .query_wasm_smart(
            govmod_single.clone(),
            &cw_proposal_single::msg::QueryMsg::ProposalHooks {},
        )
        .unwrap();
    assert_eq!(hooks.hooks.len(), 1);

    // To verify it removed the right one, lets try and remove failing counters
    // will fail as it does not exist.
    let _err = app
        .execute_contract(
            dao.clone(),
            govmod_single.clone(),
            &cw_proposal_single::msg::ExecuteMsg::RemoveProposalHook {
                address: failing_counters.to_string(),
            },
            &[],
        )
        .unwrap_err();

    // It should still have the vote hook as that has not technically failed yet
    let hooks: HooksResponse = app
        .wrap()
        .query_wasm_smart(
            govmod_single.clone(),
            &cw_proposal_single::msg::QueryMsg::VoteHooks {},
        )
        .unwrap();
    assert_eq!(hooks.hooks.len(), 2);

    // Vote on the new proposal to fail the other hook
    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        govmod_single.clone(),
        &cw_proposal_single::msg::ExecuteMsg::Vote {
            proposal_id: 2,
            vote: Vote::Yes,
        },
        &[],
    )
    .unwrap();

    // The success counters should still work
    // Query vote counter, expect 2
    let resp: CountResponse = app
        .wrap()
        .query_wasm_smart(counters.clone(), &QueryMsg::VoteCounter {})
        .unwrap();
    assert_eq!(resp.count, 2);
    // Query status changed counter, expect 2
    let resp: CountResponse = app
        .wrap()
        .query_wasm_smart(counters, &QueryMsg::StatusChangedCounter {})
        .unwrap();
    assert_eq!(resp.count, 2);

    // The contract should of removed the failing counters
    let hooks: HooksResponse = app
        .wrap()
        .query_wasm_smart(
            govmod_single.clone(),
            &cw_proposal_single::msg::QueryMsg::VoteHooks {},
        )
        .unwrap();
    assert_eq!(hooks.hooks.len(), 1);

    // To verify it removed the right one, lets try and remove failing counters
    // will fail as it does not exist.
    let _err = app
        .execute_contract(
            dao,
            govmod_single.clone(),
            &cw_proposal_single::msg::ExecuteMsg::RemoveVoteHook {
                address: failing_counters.to_string(),
            },
            &[],
        )
        .unwrap_err();

    // Verify only one hook remains for each
    let hooks: HooksResponse = app
        .wrap()
        .query_wasm_smart(
            govmod_single.clone(),
            &cw_proposal_single::msg::QueryMsg::ProposalHooks {},
        )
        .unwrap();
    assert_eq!(hooks.hooks.len(), 1);
    let hooks: HooksResponse = app
        .wrap()
        .query_wasm_smart(
            govmod_single,
            &cw_proposal_single::msg::QueryMsg::VoteHooks {},
        )
        .unwrap();
    assert_eq!(hooks.hooks.len(), 1);
}
