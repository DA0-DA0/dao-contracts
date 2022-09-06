use cosmwasm_std::{coins, to_binary, Addr, BankMsg, Binary, CosmosMsg, Decimal, Uint128, WasmMsg};
use cw20::Cw20Coin;
use cw20_staked_balance_voting::msg::ActiveThreshold;
use cw_denom::CheckedDenom;
use cw_multi_test::{next_block, App, Executor};
use cw_utils::Duration;
use indexable_hooks::{HookError, HooksResponse};
use testing::{ShouldExecute, TestSingleChoiceVote};
use voting::{
    deposit::{CheckedDepositInfo, UncheckedDepositInfo},
    pre_propose::{PreProposeInfo, ProposalCreationPolicy},
    status::Status,
    threshold::{PercentageThreshold, Threshold},
    voting::{Vote, Votes},
};

use crate::{
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
    proposal::SingleChoiceProposal,
    query::ProposalResponse,
    state::Config,
    testing::{
        execute::{
            add_proposal_hook, add_proposal_hook_should_fail, add_vote_hook,
            add_vote_hook_should_fail, close_proposal_should_fail, execute_proposal,
            execute_proposal_should_fail, instantiate_cw20_base_default, mint_natives,
            remove_proposal_hook, remove_proposal_hook_should_fail, remove_vote_hook,
            remove_vote_hook_should_fail, vote_on_proposal_should_fail,
        },
        instantiate::{
            get_default_non_token_dao_proposal_module_instantiate, get_pre_propose_info,
            instantiate_with_cw4_groups_governance,
        },
        queries::{
            query_balance_cw20, query_balance_native, query_deposit_config_and_pre_propose_module,
            query_pre_proposal_single_deposit_info,
        },
    },
    ContractError,
};

use super::{
    do_votes::do_votes_staked_balances,
    execute::{close_proposal, make_proposal, mint_cw20s, vote_on_proposal},
    instantiate::{
        get_default_token_dao_proposal_module_instantiate,
        instantiate_with_staked_balances_governance, instantiate_with_staking_active_threshold,
    },
    queries::{
        query_dao_token, query_list_proposals, query_list_proposals_reverse, query_proposal,
        query_proposal_config, query_proposal_hooks, query_single_proposal_module,
        query_vote_hooks, query_voting_module,
    },
    CREATOR_ADDR,
};

struct CommonTest {
    app: App,
    core_addr: Addr,
    proposal_module: Addr,
    gov_token: Addr,
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
        core_addr,
        proposal_module,
        gov_token,
        proposal_id,
    }
}

#[test]
fn test_simple_propose_staked_balances() {
    let CommonTest {
        app,
        core_addr: _,
        proposal_module,
        gov_token,
        proposal_id,
    } = setup_test(vec![]);

    let created = query_proposal(&app, &proposal_module, proposal_id);
    let current_block = app.block_info();

    // These values just come from the default instantiate message
    // values.
    let expected = SingleChoiceProposal {
        title: "title".to_string(),
        description: "description".to_string(),
        proposer: Addr::unchecked(CREATOR_ADDR),
        start_height: current_block.height,
        expiration: Duration::Time(604800).after(&current_block),
        min_voting_period: None,
        threshold: Threshold::ThresholdQuorum {
            quorum: PercentageThreshold::Percent(Decimal::percent(15)),
            threshold: PercentageThreshold::Majority {},
        },
        allow_revoting: false,
        total_power: Uint128::new(100_000_000),
        msgs: vec![],
        status: Status::Open,
        votes: Votes::zero(),
        created: current_block.time,
        last_updated: current_block.time,
    };

    assert_eq!(created.proposal, expected);
    assert_eq!(created.id, 1u64);

    // Check that the deposit info for this proposal looks right.
    let (_, pre_propose) = query_deposit_config_and_pre_propose_module(&app, &proposal_module);
    let deposit_response = query_pre_proposal_single_deposit_info(&app, &pre_propose, proposal_id);

    assert_eq!(deposit_response.proposer, Addr::unchecked(CREATOR_ADDR));
    assert_eq!(
        deposit_response.deposit_info,
        Some(CheckedDepositInfo {
            denom: cw_denom::CheckedDenom::Cw20(gov_token),
            amount: Uint128::new(10_000_000),
            refund_policy: voting::deposit::DepositRefundPolicy::OnlyPassed
        })
    );
}

#[test]
fn test_simple_proposal_cw4_voting() {
    let mut app = App::default();
    let instantiate = get_default_non_token_dao_proposal_module_instantiate(&mut app);
    let core_addr = instantiate_with_cw4_groups_governance(&mut app, instantiate, None);
    let proposal_module = query_single_proposal_module(&app, &core_addr);
    let id = make_proposal(&mut app, &proposal_module, CREATOR_ADDR, vec![]);

    let created = query_proposal(&app, &proposal_module, id);
    let current_block = app.block_info();

    // These values just come from the default instantiate message
    // values.
    let expected = SingleChoiceProposal {
        title: "title".to_string(),
        description: "description".to_string(),
        proposer: Addr::unchecked(CREATOR_ADDR),
        start_height: current_block.height,
        expiration: Duration::Time(604800).after(&current_block),
        min_voting_period: None,
        threshold: Threshold::ThresholdQuorum {
            threshold: PercentageThreshold::Percent(Decimal::percent(15)),
            quorum: PercentageThreshold::Majority {},
        },
        allow_revoting: false,
        total_power: Uint128::new(1),
        msgs: vec![],
        status: Status::Open,
        votes: Votes::zero(),
        created: current_block.time,
        last_updated: current_block.time,
    };

    assert_eq!(created.proposal, expected);
    assert_eq!(created.id, 1u64);

    // Check that the deposit info for this proposal looks right.
    let (_, pre_propose) = query_deposit_config_and_pre_propose_module(&app, &proposal_module);
    let deposit_response = query_pre_proposal_single_deposit_info(&app, &pre_propose, id);

    assert_eq!(deposit_response.proposer, Addr::unchecked(CREATOR_ADDR));
    assert_eq!(deposit_response.deposit_info, None,);
}

#[test]
fn test_propose_supports_stargate_messages() {
    // If we can make a proposal with a stargate message, we support
    // stargate messages in proposals.
    setup_test(vec![CosmosMsg::Stargate {
        type_url: "foo_type".to_string(),
        value: Binary::default(),
    }]);
}

/// Test that the deposit token is properly set to the voting module
/// token during instantiation.
#[test]
fn test_voting_module_token_instantiate() {
    let CommonTest {
        app,
        core_addr: _,
        proposal_module,
        gov_token,
        proposal_id,
    } = setup_test(vec![]);

    let (_, pre_propose) = query_deposit_config_and_pre_propose_module(&app, &proposal_module);
    let deposit_response = query_pre_proposal_single_deposit_info(&app, &pre_propose, proposal_id);

    let deposit_token = if let Some(CheckedDepositInfo {
        denom: CheckedDenom::Cw20(addr),
        ..
    }) = deposit_response.deposit_info
    {
        addr
    } else {
        panic!("voting module should have governance token")
    };
    assert_eq!(deposit_token, gov_token)
}

#[test]
#[should_panic(
    expected = "Error parsing into type cw4_voting::msg::QueryMsg: unknown variant `token_contract`"
)]
fn test_deposit_token_voting_module_token_fails_if_no_voting_module_token() {
    let mut app = App::default();
    let instantiate = get_default_token_dao_proposal_module_instantiate(&mut app);
    instantiate_with_cw4_groups_governance(&mut app, instantiate, None);
}

#[test]
fn test_instantiate_with_non_voting_module_cw20_deposit() {
    let mut app = App::default();
    let alt_cw20 = instantiate_cw20_base_default(&mut app);

    let mut instantiate = get_default_non_token_dao_proposal_module_instantiate(&mut app);
    // hehehehehehehehe
    instantiate.pre_propose_info = get_pre_propose_info(
        &mut app,
        Some(UncheckedDepositInfo {
            denom: voting::deposit::DepositToken::Token {
                denom: cw_denom::UncheckedDenom::Cw20(alt_cw20.to_string()),
            },
            amount: Uint128::new(10_000_000),
            refund_policy: voting::deposit::DepositRefundPolicy::OnlyPassed,
        }),
        false,
    );

    let core_addr = instantiate_with_cw4_groups_governance(&mut app, instantiate, None);
    let proposal_module = query_single_proposal_module(&app, &core_addr);
    let proposal_id = make_proposal(&mut app, &proposal_module, CREATOR_ADDR, vec![]);

    let created = query_proposal(&app, &proposal_module, proposal_id);
    let current_block = app.block_info();

    // These values just come from the default instantiate message
    // values.
    let expected = SingleChoiceProposal {
        title: "title".to_string(),
        description: "description".to_string(),
        proposer: Addr::unchecked(CREATOR_ADDR),
        start_height: current_block.height,
        expiration: Duration::Time(604800).after(&current_block),
        min_voting_period: None,
        threshold: Threshold::ThresholdQuorum {
            threshold: PercentageThreshold::Percent(Decimal::percent(15)),
            quorum: PercentageThreshold::Majority {},
        },
        allow_revoting: false,
        total_power: Uint128::new(1),
        msgs: vec![],
        status: Status::Open,
        votes: Votes::zero(),
        created: current_block.time,
        last_updated: current_block.time,
    };

    assert_eq!(created.proposal, expected);
    assert_eq!(created.id, 1u64);

    // Check that the deposit info for this proposal looks right.
    let (_, pre_propose) = query_deposit_config_and_pre_propose_module(&app, &proposal_module);
    let deposit_response = query_pre_proposal_single_deposit_info(&app, &pre_propose, proposal_id);

    assert_eq!(deposit_response.proposer, Addr::unchecked(CREATOR_ADDR));
    assert_eq!(
        deposit_response.deposit_info,
        Some(CheckedDepositInfo {
            denom: cw_denom::CheckedDenom::Cw20(alt_cw20),
            amount: Uint128::new(10_000_000),
            refund_policy: voting::deposit::DepositRefundPolicy::OnlyPassed
        })
    );
}

#[test]
fn test_proposal_message_execution() {
    let mut app = App::default();
    let mut instantiate = get_default_token_dao_proposal_module_instantiate(&mut app);
    instantiate.close_proposal_on_execution_failure = false;
    let core_addr = instantiate_with_staked_balances_governance(&mut app, instantiate, None);
    let proposal_module = query_single_proposal_module(&app, &core_addr);
    let gov_token = query_dao_token(&app, &core_addr);

    mint_cw20s(&mut app, &gov_token, &core_addr, CREATOR_ADDR, 10_000_000);
    let proposal_id = make_proposal(
        &mut app,
        &proposal_module,
        CREATOR_ADDR,
        vec![
            WasmMsg::Execute {
                contract_addr: gov_token.to_string(),
                msg: to_binary(&cw20::Cw20ExecuteMsg::Mint {
                    recipient: CREATOR_ADDR.to_string(),
                    amount: Uint128::new(10_000_000),
                })
                .unwrap(),
                funds: vec![],
            }
            .into(),
            BankMsg::Send {
                to_address: CREATOR_ADDR.to_string(),
                amount: coins(10, "ujuno"),
            }
            .into(),
        ],
    );
    let cw20_balance = query_balance_cw20(&app, &gov_token, CREATOR_ADDR);
    let native_balance = query_balance_native(&app, CREATOR_ADDR, "ujuno");
    assert_eq!(cw20_balance, Uint128::zero());
    assert_eq!(native_balance, Uint128::zero());

    vote_on_proposal(
        &mut app,
        &proposal_module,
        CREATOR_ADDR,
        proposal_id,
        Vote::Yes,
    );
    let proposal = query_proposal(&app, &proposal_module, proposal_id);
    assert_eq!(proposal.proposal.status, Status::Passed);

    // Can't use library function because we expect this to fail due
    // to insufficent balance in the bank module.
    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        proposal_module.clone(),
        &ExecuteMsg::Execute { proposal_id },
        &[],
    )
    .unwrap_err();
    let proposal = query_proposal(&app, &proposal_module, proposal_id);
    assert_eq!(proposal.proposal.status, Status::Passed);

    mint_natives(&mut app, core_addr.as_str(), coins(10, "ujuno"));
    execute_proposal(&mut app, &proposal_module, CREATOR_ADDR, proposal_id);
    let proposal = query_proposal(&app, &proposal_module, proposal_id);
    assert_eq!(proposal.proposal.status, Status::Executed);

    let cw20_balance = query_balance_cw20(&app, &gov_token, CREATOR_ADDR);
    let native_balance = query_balance_native(&app, CREATOR_ADDR, "ujuno");
    assert_eq!(cw20_balance, Uint128::new(20_000_000));
    assert_eq!(native_balance, Uint128::new(10));

    // Sneak in a check here that proposals can't be executed more
    // than once in the on close on execute config suituation.
    let err = execute_proposal_should_fail(&mut app, &proposal_module, CREATOR_ADDR, proposal_id);
    assert!(matches!(err, ContractError::NotPassed {}))
}

#[test]
fn test_proposal_close_after_expiry() {
    let CommonTest {
        mut app,
        core_addr,
        proposal_module,
        gov_token: _,
        proposal_id,
    } = setup_test(vec![BankMsg::Send {
        to_address: CREATOR_ADDR.to_string(),
        amount: coins(10, "ujuno"),
    }
    .into()]);
    mint_natives(&mut app, core_addr.as_str(), coins(10, "ujuno"));

    // Try and close the proposal. This shoudl fail as the proposal is
    // open.
    let err = close_proposal_should_fail(&mut app, &proposal_module, CREATOR_ADDR, proposal_id);
    assert!(matches!(err, ContractError::WrongCloseStatus {}));

    // Expire the proposal. Now it should be closable.
    app.update_block(|mut b| b.time = b.time.plus_seconds(604800));
    close_proposal(&mut app, &proposal_module, CREATOR_ADDR, proposal_id);
    let proposal = query_proposal(&app, &proposal_module, proposal_id);
    assert_eq!(proposal.proposal.status, Status::Closed);
}

#[test]
fn test_proposal_cant_close_after_expiry_is_passed() {
    let mut app = App::default();
    let instantiate = get_default_token_dao_proposal_module_instantiate(&mut app);
    let core_addr = instantiate_with_staked_balances_governance(
        &mut app,
        instantiate,
        Some(vec![
            Cw20Coin {
                address: "quorum".to_string(),
                amount: Uint128::new(15),
            },
            Cw20Coin {
                address: CREATOR_ADDR.to_string(),
                amount: Uint128::new(85),
            },
        ]),
    );
    let proposal_module = query_single_proposal_module(&app, &core_addr);
    let gov_token = query_dao_token(&app, &core_addr);

    mint_cw20s(&mut app, &gov_token, &core_addr, CREATOR_ADDR, 10_000_000);
    mint_natives(&mut app, core_addr.as_str(), coins(10, "ujuno"));
    let proposal_id = make_proposal(
        &mut app,
        &proposal_module,
        CREATOR_ADDR,
        vec![BankMsg::Send {
            to_address: CREATOR_ADDR.to_string(),
            amount: coins(10, "ujuno"),
        }
        .into()],
    );
    vote_on_proposal(&mut app, &proposal_module, "quorum", proposal_id, Vote::Yes);
    let proposal = query_proposal(&app, &proposal_module, proposal_id);
    assert_eq!(proposal.proposal.status, Status::Open);

    // Expire the proposal. This should pass it.
    app.update_block(|mut b| b.time = b.time.plus_seconds(604800));
    let proposal = query_proposal(&app, &proposal_module, proposal_id);
    assert_eq!(proposal.proposal.status, Status::Passed);

    // Make sure it can't be closed.
    let err = close_proposal_should_fail(&mut app, &proposal_module, CREATOR_ADDR, proposal_id);
    assert!(matches!(err, ContractError::WrongCloseStatus {}));

    // Executed proposals may not be closed.
    execute_proposal(&mut app, &proposal_module, CREATOR_ADDR, proposal_id);
    let err = close_proposal_should_fail(&mut app, &proposal_module, CREATOR_ADDR, proposal_id);
    assert!(matches!(err, ContractError::WrongCloseStatus {}));
    let balance = query_balance_native(&app, CREATOR_ADDR, "ujuno");
    assert_eq!(balance, Uint128::new(10));
    let err = close_proposal_should_fail(&mut app, &proposal_module, CREATOR_ADDR, proposal_id);
    assert!(matches!(err, ContractError::WrongCloseStatus {}));
}

#[test]
fn test_execute_no_non_passed_execution() {
    let CommonTest {
        mut app,
        core_addr,
        proposal_module,
        gov_token,
        proposal_id,
    } = setup_test(vec![BankMsg::Send {
        to_address: CREATOR_ADDR.to_string(),
        amount: coins(10, "ujuno"),
    }
    .into()]);
    mint_natives(&mut app, core_addr.as_str(), coins(100, "ujuno"));

    let err = execute_proposal_should_fail(&mut app, &proposal_module, CREATOR_ADDR, proposal_id);
    assert!(matches!(err, ContractError::NotPassed {}));

    // Expire the proposal.
    app.update_block(|mut b| b.time = b.time.plus_seconds(604800));
    let err = execute_proposal_should_fail(&mut app, &proposal_module, CREATOR_ADDR, proposal_id);
    assert!(matches!(err, ContractError::NotPassed {}));

    mint_cw20s(&mut app, &gov_token, &core_addr, CREATOR_ADDR, 10_000_000);
    let proposal_id = make_proposal(&mut app, &proposal_module, CREATOR_ADDR, vec![]);
    vote_on_proposal(
        &mut app,
        &proposal_module,
        CREATOR_ADDR,
        proposal_id,
        Vote::Yes,
    );
    execute_proposal(&mut app, &proposal_module, CREATOR_ADDR, proposal_id);
    // Can't execute more than once.
    let err = execute_proposal_should_fail(&mut app, &proposal_module, CREATOR_ADDR, proposal_id);
    assert!(matches!(err, ContractError::NotPassed {}));
}

#[test]
fn test_update_config() {
    let CommonTest {
        mut app,
        core_addr,
        proposal_module,
        gov_token: _,
        proposal_id,
    } = setup_test(vec![]);
    vote_on_proposal(
        &mut app,
        &proposal_module,
        CREATOR_ADDR,
        proposal_id,
        Vote::Yes,
    );
    execute_proposal(&mut app, &proposal_module, CREATOR_ADDR, proposal_id);
    let proposal_id = make_proposal(
        &mut app,
        &proposal_module,
        CREATOR_ADDR,
        vec![WasmMsg::Execute {
            contract_addr: proposal_module.to_string(),
            msg: to_binary(&ExecuteMsg::UpdateConfig {
                threshold: Threshold::AbsoluteCount {
                    threshold: Uint128::new(10_000),
                },
                max_voting_period: Duration::Height(6),
                min_voting_period: None,
                only_members_execute: true,
                allow_revoting: false,
                dao: core_addr.to_string(),
                close_proposal_on_execution_failure: false,
                pre_propose_info: PreProposeInfo::AnyoneMayPropose {},
            })
            .unwrap(),
            funds: vec![],
        }
        .into()],
    );
    vote_on_proposal(
        &mut app,
        &proposal_module,
        CREATOR_ADDR,
        proposal_id,
        Vote::Yes,
    );
    execute_proposal(&mut app, &proposal_module, CREATOR_ADDR, proposal_id);

    let config = query_proposal_config(&app, &proposal_module);
    assert_eq!(
        config,
        Config {
            threshold: Threshold::AbsoluteCount {
                threshold: Uint128::new(10_000)
            },
            max_voting_period: Duration::Height(6),
            min_voting_period: None,
            only_members_execute: true,
            allow_revoting: false,
            dao: core_addr.clone(),
            close_proposal_on_execution_failure: false,
            proposal_creation_policy: ProposalCreationPolicy::Anyone {}
        }
    );

    // Check that non-dao address may not update config.
    let err: ContractError = app
        .execute_contract(
            Addr::unchecked(CREATOR_ADDR),
            proposal_module,
            &&ExecuteMsg::UpdateConfig {
                threshold: Threshold::AbsoluteCount {
                    threshold: Uint128::new(10_000),
                },
                max_voting_period: Duration::Height(6),
                min_voting_period: None,
                only_members_execute: true,
                allow_revoting: false,
                dao: core_addr.to_string(),
                close_proposal_on_execution_failure: false,
                pre_propose_info: PreProposeInfo::AnyoneMayPropose {},
            },
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert!(matches!(err, ContractError::Unauthorized {}))
}

#[test]
fn test_anyone_may_propose_and_proposal_listing() {
    let mut app = App::default();
    let mut instantiate = get_default_token_dao_proposal_module_instantiate(&mut app);
    instantiate.pre_propose_info = PreProposeInfo::AnyoneMayPropose {};
    let core_addr = instantiate_with_staked_balances_governance(&mut app, instantiate, None);
    let proposal_module = query_single_proposal_module(&app, &core_addr);

    for addr in 'm'..'z' {
        let addr = addr.to_string().repeat(6);
        let proposal_id = make_proposal(&mut app, &proposal_module, &addr, vec![]);
        vote_on_proposal(
            &mut app,
            &proposal_module,
            CREATOR_ADDR,
            proposal_id,
            Vote::Yes,
        );
        // Only members can execute still.
        let err = execute_proposal_should_fail(&mut app, &proposal_module, &addr, proposal_id);
        assert!(matches!(err, ContractError::Unauthorized {}));
        execute_proposal(&mut app, &proposal_module, CREATOR_ADDR, proposal_id);
    }

    // Now that we've got all these proposals sitting around, lets
    // test that we can query them.

    let proposals_forward = query_list_proposals(&app, &proposal_module, None, None);
    let mut proposals_reverse = query_list_proposals_reverse(&app, &proposal_module, None, None);
    proposals_reverse.proposals.reverse();
    assert_eq!(proposals_reverse, proposals_forward);

    // Check the proposers and (implicitly) the ordering.
    for (index, addr) in ('m'..'z').enumerate() {
        let addr = addr.to_string().repeat(6);
        assert_eq!(
            proposals_forward.proposals[index].proposal.proposer,
            Addr::unchecked(addr)
        )
    }

    let four_and_five = query_list_proposals(&app, &proposal_module, Some(3), Some(2));
    let mut five_and_four = query_list_proposals_reverse(&app, &proposal_module, Some(6), Some(2));
    five_and_four.proposals.reverse();

    assert_eq!(five_and_four, four_and_five);
    assert_eq!(
        four_and_five.proposals[0].proposal.proposer,
        Addr::unchecked("pppppp")
    );

    let current_block = app.block_info();
    assert_eq!(
        four_and_five.proposals[0],
        ProposalResponse {
            id: 4,
            proposal: SingleChoiceProposal {
                title: "title".to_string(),
                description: "description".to_string(),
                proposer: Addr::unchecked("pppppp"),
                start_height: current_block.height,
                min_voting_period: None,
                expiration: Duration::Time(604800).after(&current_block),
                threshold: Threshold::ThresholdQuorum {
                    quorum: PercentageThreshold::Percent(Decimal::percent(15)),
                    threshold: PercentageThreshold::Majority {},
                },
                allow_revoting: false,
                total_power: Uint128::new(100_000_000),
                msgs: vec![],
                status: Status::Executed,
                votes: Votes {
                    yes: Uint128::new(100_000_000),
                    no: Uint128::zero(),
                    abstain: Uint128::zero()
                },
                created: current_block.time,
                last_updated: current_block.time,
            }
        }
    )
}

#[test]
fn test_proposal_hook_registration() {
    let CommonTest {
        mut app,
        core_addr,
        proposal_module,
        gov_token: _,
        proposal_id: _,
    } = setup_test(vec![]);

    let proposal_hooks = query_proposal_hooks(&app, &proposal_module);
    assert_eq!(
        proposal_hooks.hooks.len(),
        1,
        "pre-propose deposit module should be registered"
    );

    // non-dao may not add a hook.
    let err =
        add_proposal_hook_should_fail(&mut app, &proposal_module, CREATOR_ADDR, "proposalhook");
    assert!(matches!(err, ContractError::Unauthorized {}));

    add_proposal_hook(
        &mut app,
        &proposal_module,
        core_addr.as_str(),
        "proposalhook",
    );
    let err = add_proposal_hook_should_fail(
        &mut app,
        &proposal_module,
        core_addr.as_str(),
        "proposalhook",
    );
    assert!(matches!(
        err,
        ContractError::HookError(HookError::HookAlreadyRegistered {})
    ));

    let proposal_hooks = query_proposal_hooks(&app, &proposal_module);
    assert_eq!(proposal_hooks.hooks[1], "proposalhook".to_string());

    // Only DAO can remove proposal hooks.
    let err =
        remove_proposal_hook_should_fail(&mut app, &proposal_module, CREATOR_ADDR, "proposalhook");
    assert!(matches!(err, ContractError::Unauthorized {}));
    remove_proposal_hook(
        &mut app,
        &proposal_module,
        core_addr.as_str(),
        "proposalhook",
    );
    let proposal_hooks = query_proposal_hooks(&app, &proposal_module);
    assert_eq!(proposal_hooks.hooks.len(), 1);

    // Can not remove that which does not exist.
    let err = remove_proposal_hook_should_fail(
        &mut app,
        &proposal_module,
        core_addr.as_str(),
        "proposalhook",
    );
    assert!(matches!(
        err,
        ContractError::HookError(HookError::HookNotRegistered {})
    ));
}

#[test]
fn test_vote_hook_registration() {
    let CommonTest {
        mut app,
        core_addr,
        proposal_module,
        gov_token: _,
        proposal_id: _,
    } = setup_test(vec![]);

    let vote_hooks = query_vote_hooks(&app, &proposal_module);
    assert!(vote_hooks.hooks.is_empty(),);

    // non-dao may not add a hook.
    let err = add_vote_hook_should_fail(&mut app, &proposal_module, CREATOR_ADDR, "votehook");
    assert!(matches!(err, ContractError::Unauthorized {}));

    add_vote_hook(&mut app, &proposal_module, core_addr.as_str(), "votehook");

    let vote_hooks = query_vote_hooks(&app, &proposal_module);
    assert_eq!(
        vote_hooks,
        HooksResponse {
            hooks: vec!["votehook".to_string()]
        }
    );

    let err = add_vote_hook_should_fail(&mut app, &proposal_module, core_addr.as_str(), "votehook");
    assert!(matches!(
        err,
        ContractError::HookError(HookError::HookAlreadyRegistered {})
    ));

    let vote_hooks = query_vote_hooks(&app, &proposal_module);
    assert_eq!(vote_hooks.hooks[0], "votehook".to_string());

    // Only DAO can remove vote hooks.
    let err = remove_vote_hook_should_fail(&mut app, &proposal_module, CREATOR_ADDR, "votehook");
    assert!(matches!(err, ContractError::Unauthorized {}));
    remove_vote_hook(&mut app, &proposal_module, core_addr.as_str(), "votehook");

    let vote_hooks = query_vote_hooks(&app, &proposal_module);
    assert!(vote_hooks.hooks.is_empty(),);

    // Can not remove that which does not exist.
    let err =
        remove_vote_hook_should_fail(&mut app, &proposal_module, core_addr.as_str(), "votehook");
    assert!(matches!(
        err,
        ContractError::HookError(HookError::HookNotRegistered {})
    ));
}

#[test]
fn test_active_threshold_absolute() {
    let mut app = App::default();
    let mut instantiate = get_default_token_dao_proposal_module_instantiate(&mut app);
    instantiate.pre_propose_info = PreProposeInfo::AnyoneMayPropose {};
    let core_addr = instantiate_with_staking_active_threshold(
        &mut app,
        instantiate,
        None,
        Some(ActiveThreshold::AbsoluteCount {
            count: Uint128::new(100),
        }),
    );
    let gov_token = query_dao_token(&app, &core_addr);
    let proposal_module = query_single_proposal_module(&app, &core_addr);
    let voting_module = query_voting_module(&app, &core_addr);

    let staking_contract: Addr = app
        .wrap()
        .query_wasm_smart(
            voting_module,
            &cw20_staked_balance_voting::msg::QueryMsg::StakingContract {},
        )
        .unwrap();

    let err: ContractError = app
        .execute_contract(
            Addr::unchecked(CREATOR_ADDR),
            proposal_module.clone(),
            &ExecuteMsg::Propose {
                title: "title".to_string(),
                description: "description".to_string(),
                msgs: vec![],
                proposer: None,
            },
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert!(matches!(err, ContractError::InactiveDao {}));

    let msg = cw20::Cw20ExecuteMsg::Send {
        contract: staking_contract.to_string(),
        amount: Uint128::new(100),
        msg: to_binary(&cw20_stake::msg::ReceiveMsg::Stake {}).unwrap(),
    };
    app.execute_contract(Addr::unchecked(CREATOR_ADDR), gov_token, &msg, &[])
        .unwrap();
    app.update_block(next_block);

    // Proposal creation now works as tokens have been staked to reach
    // active threshold.
    make_proposal(&mut app, &proposal_module, CREATOR_ADDR, vec![]);

    // Unstake some tokens to make it inactive again.
    let msg = cw20_stake::msg::ExecuteMsg::Unstake {
        amount: Uint128::new(50),
    };
    app.execute_contract(Addr::unchecked(CREATOR_ADDR), staking_contract, &msg, &[])
        .unwrap();
    app.update_block(next_block);

    let err: ContractError = app
        .execute_contract(
            Addr::unchecked(CREATOR_ADDR),
            proposal_module.clone(),
            &ExecuteMsg::Propose {
                title: "title".to_string(),
                description: "description".to_string(),
                msgs: vec![],
                proposer: None,
            },
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert!(matches!(err, ContractError::InactiveDao {}));
}

#[test]
fn test_active_threshold_percent() {
    let mut app = App::default();
    let mut instantiate = get_default_token_dao_proposal_module_instantiate(&mut app);
    instantiate.pre_propose_info = PreProposeInfo::AnyoneMayPropose {};
    let core_addr = instantiate_with_staking_active_threshold(
        &mut app,
        instantiate,
        None,
        Some(ActiveThreshold::Percentage {
            percent: Decimal::percent(20),
        }),
    );
    let gov_token = query_dao_token(&app, &core_addr);
    let proposal_module = query_single_proposal_module(&app, &core_addr);
    let voting_module = query_voting_module(&app, &core_addr);

    let staking_contract: Addr = app
        .wrap()
        .query_wasm_smart(
            voting_module,
            &cw20_staked_balance_voting::msg::QueryMsg::StakingContract {},
        )
        .unwrap();

    let err: ContractError = app
        .execute_contract(
            Addr::unchecked(CREATOR_ADDR),
            proposal_module.clone(),
            &ExecuteMsg::Propose {
                title: "title".to_string(),
                description: "description".to_string(),
                msgs: vec![],
                proposer: None,
            },
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert!(matches!(err, ContractError::InactiveDao {}));

    let msg = cw20::Cw20ExecuteMsg::Send {
        contract: staking_contract.to_string(),
        amount: Uint128::new(20_000_000),
        msg: to_binary(&cw20_stake::msg::ReceiveMsg::Stake {}).unwrap(),
    };
    app.execute_contract(Addr::unchecked(CREATOR_ADDR), gov_token, &msg, &[])
        .unwrap();
    app.update_block(next_block);

    // Proposal creation now works as tokens have been staked to reach
    // active threshold.
    make_proposal(&mut app, &proposal_module, CREATOR_ADDR, vec![]);

    // Unstake some tokens to make it inactive again.
    let msg = cw20_stake::msg::ExecuteMsg::Unstake {
        amount: Uint128::new(1), // Only one is needed as we're right
                                 // on the edge. :)
    };
    app.execute_contract(Addr::unchecked(CREATOR_ADDR), staking_contract, &msg, &[])
        .unwrap();
    app.update_block(next_block);

    let err: ContractError = app
        .execute_contract(
            Addr::unchecked(CREATOR_ADDR),
            proposal_module.clone(),
            &ExecuteMsg::Propose {
                title: "title".to_string(),
                description: "description".to_string(),
                msgs: vec![],
                proposer: None,
            },
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert!(matches!(err, ContractError::InactiveDao {}));
}

#[test]
#[should_panic(
    expected = "min_voting_period and max_voting_period must have the same units (height or time)"
)]
fn test_min_duration_unit_missmatch() {
    let mut app = App::default();
    let mut instantiate = get_default_token_dao_proposal_module_instantiate(&mut app);
    instantiate.min_voting_period = Some(Duration::Height(10));
    instantiate_with_staked_balances_governance(&mut app, instantiate, None);
}

#[test]
#[should_panic(expected = "Min voting period must be less than or equal to max voting period")]
fn test_min_duration_larger_than_proposal_duration() {
    let mut app = App::default();
    let mut instantiate = get_default_token_dao_proposal_module_instantiate(&mut app);
    instantiate.min_voting_period = Some(Duration::Time(604801));
    instantiate_with_staked_balances_governance(&mut app, instantiate, None);
}

#[test]
fn test_min_voting_period_no_early_pass() {
    let mut app = App::default();
    let mut instantiate = get_default_token_dao_proposal_module_instantiate(&mut app);
    instantiate.min_voting_period = Some(Duration::Height(10));
    instantiate.max_voting_period = Duration::Height(100);
    let core_addr = instantiate_with_staked_balances_governance(&mut app, instantiate, None);
    let gov_token = query_dao_token(&app, &core_addr);
    let proposal_module = query_single_proposal_module(&app, &core_addr);

    mint_cw20s(&mut app, &gov_token, &core_addr, CREATOR_ADDR, 10_000_000);
    let proposal_id = make_proposal(&mut app, &proposal_module, CREATOR_ADDR, vec![]);
    vote_on_proposal(
        &mut app,
        &proposal_module,
        CREATOR_ADDR,
        proposal_id,
        Vote::Yes,
    );
    let proposal_response = query_proposal(&app, &proposal_module, proposal_id);
    assert_eq!(proposal_response.proposal.status, Status::Open);

    app.update_block(|mut block| block.height += 10);
    let proposal_response = query_proposal(&app, &proposal_module, proposal_id);
    assert_eq!(proposal_response.proposal.status, Status::Passed);
}

// Setting the min duration the same as the proposal duration just
// means that proposals cant close early.
#[test]
fn test_min_duration_same_as_proposal_duration() {
    let mut app = App::default();
    let mut instantiate = get_default_token_dao_proposal_module_instantiate(&mut app);
    instantiate.min_voting_period = Some(Duration::Height(100));
    instantiate.max_voting_period = Duration::Height(100);
    let core_addr = instantiate_with_staked_balances_governance(
        &mut app,
        instantiate,
        Some(vec![
            Cw20Coin {
                address: "ekez".to_string(),
                amount: Uint128::new(10),
            },
            Cw20Coin {
                address: "whale".to_string(),
                amount: Uint128::new(90),
            },
        ]),
    );
    let gov_token = query_dao_token(&app, &core_addr);
    let proposal_module = query_single_proposal_module(&app, &core_addr);

    mint_cw20s(&mut app, &gov_token, &core_addr, "ekez", 10_000_000);
    let proposal_id = make_proposal(&mut app, &proposal_module, "ekez", vec![]);

    // Whale votes yes. Normally the proposal would just pass and ekez
    // would be out of luck.
    vote_on_proposal(&mut app, &proposal_module, "whale", proposal_id, Vote::Yes);
    vote_on_proposal(&mut app, &proposal_module, "ekez", proposal_id, Vote::No);

    app.update_block(|mut b| b.height += 100);
    let proposal_response = query_proposal(&app, &proposal_module, proposal_id);
    assert_eq!(proposal_response.proposal.status, Status::Passed);
}

#[test]
fn test_revoting_playthrough() {
    let mut app = App::default();
    let mut instantiate = get_default_token_dao_proposal_module_instantiate(&mut app);
    instantiate.allow_revoting = true;
    let core_addr = instantiate_with_staked_balances_governance(&mut app, instantiate, None);
    let gov_token = query_dao_token(&app, &core_addr);
    let proposal_module = query_single_proposal_module(&app, &core_addr);

    mint_cw20s(&mut app, &gov_token, &core_addr, CREATOR_ADDR, 10_000_000);
    let proposal_id = make_proposal(&mut app, &proposal_module, CREATOR_ADDR, vec![]);

    // Vote and change our minds a couple times.
    vote_on_proposal(
        &mut app,
        &proposal_module,
        CREATOR_ADDR,
        proposal_id,
        Vote::Yes,
    );
    let proposal_response = query_proposal(&app, &proposal_module, proposal_id);
    assert_eq!(proposal_response.proposal.status, Status::Open);

    vote_on_proposal(
        &mut app,
        &proposal_module,
        CREATOR_ADDR,
        proposal_id,
        Vote::No,
    );
    let proposal_response = query_proposal(&app, &proposal_module, proposal_id);
    assert_eq!(proposal_response.proposal.status, Status::Open);

    vote_on_proposal(
        &mut app,
        &proposal_module,
        CREATOR_ADDR,
        proposal_id,
        Vote::Yes,
    );
    let proposal_response = query_proposal(&app, &proposal_module, proposal_id);
    assert_eq!(proposal_response.proposal.status, Status::Open);

    // Can't cast the same vote more than once.
    let err = vote_on_proposal_should_fail(
        &mut app,
        &proposal_module,
        CREATOR_ADDR,
        proposal_id,
        Vote::Yes,
    );
    assert!(matches!(err, ContractError::AlreadyCast {}));

    // Expire the proposal allowing the votes to be tallied.
    app.update_block(|mut b| b.time = b.time.plus_seconds(604800));
    let proposal_response = query_proposal(&app, &proposal_module, proposal_id);
    assert_eq!(proposal_response.proposal.status, Status::Passed);
    execute_proposal(&mut app, &proposal_module, CREATOR_ADDR, proposal_id);

    // Can't vote once the proposal is passed.
    let err = vote_on_proposal_should_fail(
        &mut app,
        &proposal_module,
        CREATOR_ADDR,
        proposal_id,
        Vote::Yes,
    );
    assert!(matches!(err, ContractError::NotOpen { .. }));
}

/// Tests that revoting is stored at a per-proposal level. Proposals
/// created while revoting is enabled should not have it disabled if a
/// config change turns if off.
#[test]
fn test_allow_revoting_config_changes() {
    let mut app = App::default();
    let mut instantiate = get_default_token_dao_proposal_module_instantiate(&mut app);
    instantiate.allow_revoting = true;
    let core_addr = instantiate_with_staked_balances_governance(&mut app, instantiate, None);
    let gov_token = query_dao_token(&app, &core_addr);
    let proposal_module = query_single_proposal_module(&app, &core_addr);

    mint_cw20s(&mut app, &gov_token, &core_addr, CREATOR_ADDR, 10_000_000);
    let revoting_proposal = make_proposal(&mut app, &proposal_module, CREATOR_ADDR, vec![]);

    // Update the config of the proposal module to disable revoting.
    app.execute_contract(
        core_addr.clone(),
        proposal_module.clone(),
        &ExecuteMsg::UpdateConfig {
            threshold: Threshold::ThresholdQuorum {
                quorum: PercentageThreshold::Percent(Decimal::percent(15)),
                threshold: PercentageThreshold::Majority {},
            },
            max_voting_period: Duration::Height(10),
            min_voting_period: None,
            only_members_execute: true,
            // Turn off revoting.
            allow_revoting: false,
            dao: core_addr.to_string(),
            close_proposal_on_execution_failure: false,
            pre_propose_info: PreProposeInfo::AnyoneMayPropose {},
        },
        &[],
    )
    .unwrap();

    let no_revoting_proposal = make_proposal(&mut app, &proposal_module, CREATOR_ADDR, vec![]);

    vote_on_proposal(
        &mut app,
        &proposal_module,
        CREATOR_ADDR,
        revoting_proposal,
        Vote::Yes,
    );
    vote_on_proposal(
        &mut app,
        &proposal_module,
        CREATOR_ADDR,
        no_revoting_proposal,
        Vote::Yes,
    );

    // Proposal without revoting should have passed.
    let proposal_resp = query_proposal(&app, &proposal_module, no_revoting_proposal);
    assert_eq!(proposal_resp.proposal.status, Status::Passed);

    // Proposal with revoting should not have passed.
    let proposal_resp = query_proposal(&app, &proposal_module, revoting_proposal);
    assert_eq!(proposal_resp.proposal.status, Status::Open);

    // Can not vote again on the no revoting proposal.
    let err = vote_on_proposal_should_fail(
        &mut app,
        &proposal_module,
        CREATOR_ADDR,
        no_revoting_proposal,
        Vote::No,
    );
    assert!(matches!(err, ContractError::NotOpen { .. }));

    // Can change vote on the revoting proposal.
    vote_on_proposal(
        &mut app,
        &proposal_module,
        CREATOR_ADDR,
        revoting_proposal,
        Vote::No,
    );
    // Expire the revoting proposal and close it.
    app.update_block(|mut b| b.time = b.time.plus_seconds(604800));
    close_proposal(&mut app, &proposal_module, CREATOR_ADDR, revoting_proposal);
}

/// Tests a simple three of five multisig configuration.
#[test]
fn test_three_of_five_multisig() {
    let mut app = App::default();
    let mut instantiate = get_default_non_token_dao_proposal_module_instantiate(&mut app);
    instantiate.threshold = Threshold::AbsoluteCount {
        threshold: Uint128::new(3),
    };
    instantiate.pre_propose_info = PreProposeInfo::AnyoneMayPropose {};
    let core_addr = instantiate_with_cw4_groups_governance(
        &mut app,
        instantiate,
        Some(vec![
            Cw20Coin {
                address: "one".to_string(),
                amount: Uint128::new(1),
            },
            Cw20Coin {
                address: "two".to_string(),
                amount: Uint128::new(1),
            },
            Cw20Coin {
                address: "three".to_string(),
                amount: Uint128::new(1),
            },
            Cw20Coin {
                address: "four".to_string(),
                amount: Uint128::new(1),
            },
            Cw20Coin {
                address: "five".to_string(),
                amount: Uint128::new(1),
            },
        ]),
    );

    let core_state: cw_core::query::DumpStateResponse = app
        .wrap()
        .query_wasm_smart(core_addr, &cw_core::msg::QueryMsg::DumpState {})
        .unwrap();
    let proposal_module = core_state
        .proposal_modules
        .into_iter()
        .next()
        .unwrap()
        .address;

    let proposal_id = make_proposal(&mut app, &proposal_module, CREATOR_ADDR, vec![]);

    vote_on_proposal(&mut app, &proposal_module, "one", proposal_id, Vote::Yes);
    vote_on_proposal(&mut app, &proposal_module, "two", proposal_id, Vote::Yes);

    // Make sure it doesn't pass early.
    let proposal: ProposalResponse = query_proposal(&app, &proposal_module, 1);
    assert_eq!(proposal.proposal.status, Status::Open);

    vote_on_proposal(&mut app, &proposal_module, "three", proposal_id, Vote::Yes);

    let proposal: ProposalResponse = query_proposal(&app, &proposal_module, 1);
    assert_eq!(proposal.proposal.status, Status::Passed);

    execute_proposal(&mut app, &proposal_module, "four", proposal_id);

    let proposal: ProposalResponse = query_proposal(&app, &proposal_module, 1);
    assert_eq!(proposal.proposal.status, Status::Executed);

    // Make another proposal which we'll reject.
    let proposal_id = make_proposal(&mut app, &proposal_module, "one", vec![]);

    vote_on_proposal(&mut app, &proposal_module, "one", proposal_id, Vote::Yes);
    vote_on_proposal(&mut app, &proposal_module, "two", proposal_id, Vote::No);
    vote_on_proposal(&mut app, &proposal_module, "three", proposal_id, Vote::No);
    vote_on_proposal(&mut app, &proposal_module, "four", proposal_id, Vote::No);

    let proposal = query_proposal(&app, &proposal_module, proposal_id);
    assert_eq!(proposal.proposal.status, Status::Rejected);

    close_proposal(&mut app, &proposal_module, "four", proposal_id);
    let proposal = query_proposal(&app, &proposal_module, proposal_id);
    assert_eq!(proposal.proposal.status, Status::Closed);
}

#[test]
fn test_three_of_five_multisig_revoting() {
    let mut app = App::default();
    let mut instantiate = get_default_non_token_dao_proposal_module_instantiate(&mut app);
    instantiate.threshold = Threshold::AbsoluteCount {
        threshold: Uint128::new(3),
    };
    instantiate.allow_revoting = true;
    instantiate.pre_propose_info = PreProposeInfo::AnyoneMayPropose {};
    let core_addr = instantiate_with_cw4_groups_governance(
        &mut app,
        instantiate,
        Some(vec![
            Cw20Coin {
                address: "one".to_string(),
                amount: Uint128::new(1),
            },
            Cw20Coin {
                address: "two".to_string(),
                amount: Uint128::new(1),
            },
            Cw20Coin {
                address: "three".to_string(),
                amount: Uint128::new(1),
            },
            Cw20Coin {
                address: "four".to_string(),
                amount: Uint128::new(1),
            },
            Cw20Coin {
                address: "five".to_string(),
                amount: Uint128::new(1),
            },
        ]),
    );

    let core_state: cw_core::query::DumpStateResponse = app
        .wrap()
        .query_wasm_smart(core_addr, &cw_core::msg::QueryMsg::DumpState {})
        .unwrap();
    let proposal_module = core_state
        .proposal_modules
        .into_iter()
        .next()
        .unwrap()
        .address;

    let proposal_id = make_proposal(&mut app, &proposal_module, CREATOR_ADDR, vec![]);

    vote_on_proposal(&mut app, &proposal_module, "one", proposal_id, Vote::Yes);
    vote_on_proposal(&mut app, &proposal_module, "two", proposal_id, Vote::Yes);

    // Make sure it doesn't pass early.
    let proposal: ProposalResponse = query_proposal(&app, &proposal_module, proposal_id);
    assert_eq!(proposal.proposal.status, Status::Open);

    vote_on_proposal(&mut app, &proposal_module, "three", proposal_id, Vote::Yes);

    // Revoting is enabled so the proposal is still open.
    let proposal: ProposalResponse = query_proposal(&app, &proposal_module, proposal_id);
    assert_eq!(proposal.proposal.status, Status::Open);

    // Change our minds.
    vote_on_proposal(&mut app, &proposal_module, "one", proposal_id, Vote::No);
    vote_on_proposal(&mut app, &proposal_module, "two", proposal_id, Vote::No);

    let err =
        vote_on_proposal_should_fail(&mut app, &proposal_module, "two", proposal_id, Vote::No);
    assert!(matches!(err, ContractError::AlreadyCast {}));

    // Expire the revoting proposal and close it.
    app.update_block(|mut b| b.time = b.time.plus_seconds(604800));
    let proposal: ProposalResponse = query_proposal(&app, &proposal_module, proposal_id);
    assert_eq!(proposal.proposal.status, Status::Rejected);
}

/// Tests that absolute count style thresholds work with token style
/// voting.
#[test]
fn test_absolute_count_threshold_non_multisig() {
    do_votes_staked_balances(
        vec![
            TestSingleChoiceVote {
                voter: "one".to_string(),
                position: Vote::Yes,
                weight: Uint128::new(10),
                should_execute: ShouldExecute::Yes,
            },
            TestSingleChoiceVote {
                voter: "two".to_string(),
                position: Vote::No,
                weight: Uint128::new(200),
                should_execute: ShouldExecute::Yes,
            },
            TestSingleChoiceVote {
                voter: "three".to_string(),
                position: Vote::Yes,
                weight: Uint128::new(1),
                should_execute: ShouldExecute::Yes,
            },
        ],
        Threshold::AbsoluteCount {
            threshold: Uint128::new(11),
        },
        Status::Passed,
        None,
    );
}

/// Tests that we do not overflow when faced with really high token /
/// vote supply.
#[test]
fn test_large_absolute_count_threshold() {
    do_votes_staked_balances(
        vec![
            // Instant rejection after this.
            TestSingleChoiceVote {
                voter: "two".to_string(),
                position: Vote::No,
                weight: Uint128::new(1),
                should_execute: ShouldExecute::Yes,
            },
            TestSingleChoiceVote {
                voter: "one".to_string(),
                position: Vote::Yes,
                weight: Uint128::new(u128::MAX - 1),
                should_execute: ShouldExecute::No,
            },
        ],
        Threshold::AbsoluteCount {
            threshold: Uint128::new(u128::MAX),
        },
        Status::Rejected,
        None,
    );

    do_votes_staked_balances(
        vec![
            TestSingleChoiceVote {
                voter: "one".to_string(),
                position: Vote::Yes,
                weight: Uint128::new(u128::MAX - 1),
                should_execute: ShouldExecute::Yes,
            },
            TestSingleChoiceVote {
                voter: "two".to_string(),
                position: Vote::No,
                weight: Uint128::new(1),
                should_execute: ShouldExecute::Yes,
            },
        ],
        Threshold::AbsoluteCount {
            threshold: Uint128::new(u128::MAX),
        },
        Status::Rejected,
        None,
    );
}

#[test]
fn test_proposal_count_initialized_to_zero() {
    let mut app = App::default();
    let pre_propose_info = get_pre_propose_info(&mut app, None, false);
    let core_addr = instantiate_with_staked_balances_governance(
        &mut app,
        InstantiateMsg {
            threshold: Threshold::ThresholdQuorum {
                threshold: PercentageThreshold::Majority {},
                quorum: PercentageThreshold::Percent(Decimal::percent(10)),
            },
            max_voting_period: Duration::Height(10),
            min_voting_period: None,
            only_members_execute: true,
            allow_revoting: false,
            pre_propose_info,
            close_proposal_on_execution_failure: true,
        },
        Some(vec![
            Cw20Coin {
                address: "ekez".to_string(),
                amount: Uint128::new(10),
            },
            Cw20Coin {
                address: "innactive".to_string(),
                amount: Uint128::new(90),
            },
        ]),
    );

    let core_state: cw_core::query::DumpStateResponse = app
        .wrap()
        .query_wasm_smart(core_addr, &cw_core::msg::QueryMsg::DumpState {})
        .unwrap();
    let proposal_modules = core_state.proposal_modules;

    assert_eq!(proposal_modules.len(), 1);
    let proposal_single = proposal_modules.into_iter().next().unwrap().address;

    let proposal_count: u64 = app
        .wrap()
        .query_wasm_smart(proposal_single, &QueryMsg::ProposalCount {})
        .unwrap();
    assert_eq!(proposal_count, 0);
}

// - Update deposit module.
// - Old deposits refunded on deposit module update.
// - Withdraw from deposit module that has been removed.
// - Test you can not remove the hook for the pre-propose module..
