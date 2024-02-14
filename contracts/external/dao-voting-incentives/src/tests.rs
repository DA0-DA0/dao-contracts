use std::vec;

use cosmwasm_std::{
    testing::{mock_dependencies, mock_env},
    to_json_binary, Addr, Binary, Coin, CosmosMsg, Uint128, WasmMsg,
};

use cw20::{BalanceResponse, Cw20Coin, Cw20QueryMsg, Cw20ReceiveMsg};
use cw_denom::{CheckedDenom, UncheckedDenom};
use cw_multi_test::{error::AnyResult, App, AppBuilder, AppResponse, Executor};
use cw_utils::Expiration;
use dao_testing::{
    contracts::{cw20_base_contract, dao_voting_incentives_contract, proposal_single_contract},
    helpers::instantiate_with_cw4_groups_governance,
};
use dao_voting::{proposal::SingleChoiceProposeMsg, threshold::Threshold};

use crate::{
    contract::{migrate, CONTRACT_NAME, CONTRACT_VERSION},
    msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, RewardResponse},
    state::Config,
};

const ADMIN: &str = "admin";
const ADDR1: &str = "addr1";
const ADDR2: &str = "addr2";
const DENOM: &str = "juno";

struct Context {
    app: App,
    cw20_addr: Addr,
    proposal_single_addr: Addr,
    dao_addr: Addr,
    dao_voting_incentives_code_id: u64,
}

fn get_context() -> Context {
    // Set up app with native balances
    let mut app = AppBuilder::default().build(|router, _, storage| {
        router
            .bank
            .init_balance(
                storage,
                &Addr::unchecked(ADMIN),
                vec![Coin {
                    denom: DENOM.to_string(),
                    amount: Uint128::new(1000),
                }],
            )
            .unwrap();
    });

    // Set up cw20 with balances
    let cw20_code_id = app.store_code(cw20_base_contract());
    let cw20_addr = app
        .instantiate_contract(
            cw20_code_id,
            Addr::unchecked(ADMIN),
            &cw20_base::msg::InstantiateMsg {
                name: "cw20 token".to_string(),
                symbol: "cwtoken".to_string(),
                decimals: 6,
                initial_balances: vec![Cw20Coin {
                    address: ADMIN.to_string(),
                    amount: Uint128::new(1000),
                }],
                mint: None,
                marketing: None,
            },
            &[],
            "cw20-base",
            None,
        )
        .unwrap();

    // Set up dao
    let proposal_single_code_id = app.store_code(proposal_single_contract());
    let dao_addr = instantiate_with_cw4_groups_governance(
        &mut app,
        proposal_single_code_id,
        to_json_binary(&dao_proposal_single::msg::InstantiateMsg {
            threshold: Threshold::AbsolutePercentage {
                percentage: dao_voting::threshold::PercentageThreshold::Majority {},
            },
            max_voting_period: cw_utils::Duration::Height(10u64),
            min_voting_period: None,
            only_members_execute: false,
            allow_revoting: false,
            pre_propose_info: dao_voting::pre_propose::PreProposeInfo::AnyoneMayPropose {},
            close_proposal_on_execution_failure: true,
            veto: None,
        })
        .unwrap(),
        Some(vec![
            Cw20Coin {
                address: ADDR1.to_string(),
                amount: Uint128::one(),
            },
            Cw20Coin {
                address: ADDR2.to_string(),
                amount: Uint128::one(),
            },
        ]),
    );

    // Get proposal single addr
    let proposal_modules: Vec<dao_interface::state::ProposalModule> = app
        .wrap()
        .query_wasm_smart(
            dao_addr.clone(),
            &dao_interface::msg::QueryMsg::ProposalModules {
                start_after: None,
                limit: Some(1u32),
            },
        )
        .unwrap();
    assert!(!proposal_modules.is_empty());
    let proposal_single_addr = proposal_modules.first().unwrap().address.clone();

    // Set up dao voting incentives code id
    let dao_voting_incentives_code_id = app.store_code(dao_voting_incentives_contract());

    Context {
        app,
        cw20_addr,
        dao_addr,
        dao_voting_incentives_code_id,
        proposal_single_addr,
    }
}

fn vote_yes_on_proposal(context: &mut Context, proposal_id: u64) -> Vec<AnyResult<AppResponse>> {
    vec![
        context.app.execute_contract(
            Addr::unchecked(ADDR1),
            context.proposal_single_addr.clone(),
            &dao_proposal_single::msg::ExecuteMsg::Vote {
                proposal_id,
                vote: dao_voting::voting::Vote::Yes,
                rationale: None,
            },
            &[],
        ),
        context.app.execute_contract(
            Addr::unchecked(ADDR2),
            context.proposal_single_addr.clone(),
            &dao_proposal_single::msg::ExecuteMsg::Vote {
                proposal_id,
                vote: dao_voting::voting::Vote::Yes,
                rationale: None,
            },
            &[],
        ),
    ]
}

fn execute_proposal(context: &mut Context, proposal_id: u64) {
    context
        .app
        .execute_contract(
            Addr::unchecked(ADDR1),
            context.proposal_single_addr.clone(),
            &dao_proposal_single::msg::ExecuteMsg::Execute { proposal_id },
            &[],
        )
        .unwrap();
}

#[test]
pub fn test_instantiate_validation() {
    let mut context = get_context();

    // Check already expired is error
    let result = context.app.instantiate_contract(
        context.dao_voting_incentives_code_id,
        Addr::unchecked(ADMIN),
        &InstantiateMsg {
            owner: context.dao_addr.to_string(),
            denom: UncheckedDenom::Native(DENOM.to_string()),
            expiration: Expiration::AtHeight(1u64),
        },
        &[],
        "dao_voting_incentives".to_string(),
        None,
    );
    assert!(result.is_err());

    // Check expiration never is error
    let result = context.app.instantiate_contract(
        context.dao_voting_incentives_code_id,
        Addr::unchecked(ADMIN),
        &InstantiateMsg {
            owner: context.dao_addr.to_string(),
            denom: UncheckedDenom::Native(DENOM.to_string()),
            expiration: Expiration::Never {},
        },
        &[],
        "dao_voting_incentives".to_string(),
        None,
    );
    assert!(result.is_err());
}

#[test]
pub fn test_hooks() {
    let mut context = get_context();

    // Create the voting incentives contract for native
    // The expiration is 10 blocks from start (12345 height)
    let dao_voting_incentives_addr = context
        .app
        .instantiate_contract(
            context.dao_voting_incentives_code_id,
            Addr::unchecked(ADMIN),
            &InstantiateMsg {
                owner: context.dao_addr.to_string(),
                denom: UncheckedDenom::Native(DENOM.to_string()),
                expiration: Expiration::AtHeight(12355u64),
            },
            &[],
            "dao_voting_incentives".to_string(),
            None,
        )
        .unwrap();

    // Also create a parallel voting incentives contract for cw20
    let dao_voting_incentives_cw20_addr = context
        .app
        .instantiate_contract(
            context.dao_voting_incentives_code_id,
            Addr::unchecked(ADMIN),
            &InstantiateMsg {
                owner: context.dao_addr.to_string(),
                denom: UncheckedDenom::Cw20(context.cw20_addr.to_string()),
                expiration: Expiration::AtHeight(12355u64),
            },
            &[],
            "dao_voting_incentives_cw20".to_string(),
            None,
        )
        .unwrap();

    context.app.update_block(|x| x.height += 1);

    // Execute fails - unauthorized
    let result = context.app.execute_contract(
        Addr::unchecked(ADMIN),
        dao_voting_incentives_addr.clone(),
        &ExecuteMsg::VoteHook(dao_hooks::vote::VoteHookMsg::NewVote {
            proposal_id: 1u64,
            voter: ADMIN.to_string(),
            vote: "a fake vote".to_string(),
        }),
        &[],
    );
    assert!(result.is_err());

    // Fund the incentives contracts for 1000
    context
        .app
        .send_tokens(
            Addr::unchecked(ADMIN),
            dao_voting_incentives_addr.clone(),
            &[Coin {
                denom: DENOM.to_string(),
                amount: Uint128::new(1000),
            }],
        )
        .unwrap();
    let result = context.app.execute_contract(
        Addr::unchecked(ADMIN),
        context.cw20_addr.clone(),
        &cw20::Cw20ExecuteMsg::Send {
            contract: dao_voting_incentives_cw20_addr.to_string(),
            amount: Uint128::new(1000),
            msg: Binary::default(),
        },
        &[],
    );
    assert!(result.is_ok());

    // Assert the cw20 voting incentives do not accept a random cw20 token
    let result = context.app.execute_contract(
        Addr::unchecked(ADMIN),
        dao_voting_incentives_cw20_addr.clone(),
        &ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: ADMIN.to_string(),
            amount: Uint128::new(1000),
            msg: Binary::default(),
        }),
        &[],
    );
    assert!(result.is_err());

    // Propose adding both hooks
    let result = context.app.execute_contract(
        Addr::unchecked(ADDR1),
        context.proposal_single_addr.clone(),
        &dao_proposal_single::msg::ExecuteMsg::Propose(SingleChoiceProposeMsg {
            title: "Add vote hooks".to_string(),
            description: "Adding 2 voting hooks to test the dao_voting_incentives contract"
                .to_string(),
            msgs: vec![
                CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: context.proposal_single_addr.to_string(),
                    msg: to_json_binary(&dao_proposal_single::msg::ExecuteMsg::AddVoteHook {
                        address: dao_voting_incentives_addr.to_string(),
                    })
                    .unwrap(),
                    funds: vec![],
                }),
                CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: context.proposal_single_addr.to_string(),
                    msg: to_json_binary(&dao_proposal_single::msg::ExecuteMsg::AddVoteHook {
                        address: dao_voting_incentives_cw20_addr.to_string(),
                    })
                    .unwrap(),
                    funds: vec![],
                }),
            ],
            proposer: None,
        }),
        &[],
    );
    assert!(result.is_ok());

    // Vote and execute the proposal to add the vote hooks
    let results = vote_yes_on_proposal(&mut context, 1u64);
    for result in results {
        assert!(result.is_ok());
    }
    execute_proposal(&mut context, 1u64);

    // Query for the newly-established hooks
    let result: cw_hooks::HooksResponse = context
        .app
        .wrap()
        .query_wasm_smart(
            context.proposal_single_addr.clone(),
            &dao_proposal_single::msg::QueryMsg::VoteHooks {},
        )
        .unwrap();
    assert!(result
        .hooks
        .contains(&dao_voting_incentives_addr.to_string()));
    assert!(result
        .hooks
        .contains(&dao_voting_incentives_cw20_addr.to_string()));

    // Create a new proposal
    context
        .app
        .execute_contract(
            Addr::unchecked(ADDR1),
            context.proposal_single_addr.clone(),
            &dao_proposal_single::msg::ExecuteMsg::Propose(SingleChoiceProposeMsg {
                title: "Test proposal".to_string(),
                description: "Testing".to_string(),
                msgs: vec![],
                proposer: None,
            }),
            &[],
        )
        .unwrap();

    // Trigger a vote hook
    let results = vote_yes_on_proposal(&mut context, 2u64);
    for result in results {
        assert!(result.is_ok());
    }

    // Assert that the vote hook has incremented vote counts
    let votes: Uint128 = context
        .app
        .wrap()
        .query_wasm_smart(
            dao_voting_incentives_addr.clone(),
            &QueryMsg::Votes {
                address: ADDR1.to_string(),
            },
        )
        .unwrap();
    assert_eq!(votes, Uint128::one());
    let votes: Uint128 = context
        .app
        .wrap()
        .query_wasm_smart(
            dao_voting_incentives_addr.clone(),
            &QueryMsg::Votes {
                address: ADDR2.to_string(),
            },
        )
        .unwrap();
    assert_eq!(votes, Uint128::one());
    let votes: Uint128 = context
        .app
        .wrap()
        .query_wasm_smart(
            dao_voting_incentives_cw20_addr.clone(),
            &QueryMsg::Votes {
                address: ADDR1.to_string(),
            },
        )
        .unwrap();
    assert_eq!(votes, Uint128::one());
    let votes: Uint128 = context
        .app
        .wrap()
        .query_wasm_smart(
            dao_voting_incentives_cw20_addr.clone(),
            &QueryMsg::Votes {
                address: ADDR2.to_string(),
            },
        )
        .unwrap();
    assert_eq!(votes, Uint128::one());
    let config: Config = context
        .app
        .wrap()
        .query_wasm_smart(dao_voting_incentives_addr.clone(), &QueryMsg::Config {})
        .unwrap();
    assert_eq!(config.total_votes, Uint128::new(2));

    // Blocks have passed the voting incentives' expirations
    context.app.update_block(|x| x.height += 100);

    // Creating another proposal and voting should still succeed but unregister these vote hooks
    context
        .app
        .execute_contract(
            Addr::unchecked(ADDR1),
            context.proposal_single_addr.clone(),
            &dao_proposal_single::msg::ExecuteMsg::Propose(SingleChoiceProposeMsg {
                title: "Test proposal".to_string(),
                description: "Testing".to_string(),
                msgs: vec![],
                proposer: None,
            }),
            &[],
        )
        .unwrap();
    let results = vote_yes_on_proposal(&mut context, 3u64);
    for (i, result) in results.iter().enumerate() {
        assert!(result.is_ok());
        if i == 0 {
            // Vote hooks should unregister on the first instance of expiration
            let events = &result.as_ref().unwrap().events;
            assert!(events.iter().any(|x| x
                .attributes
                .iter()
                .any(|y| y.key == "removed_vote_hook"
                    && y.value == format!("{0}:0", dao_voting_incentives_addr.clone()))));
            assert!(events.iter().any(|x| x
                .attributes
                .iter()
                .any(|y| y.key == "removed_vote_hook"
                    && y.value == format!("{0}:1", dao_voting_incentives_cw20_addr.clone()))));
        }
    }

    // Expire the vote hooks
    let result = context.app.execute_contract(
        Addr::unchecked(ADMIN),
        dao_voting_incentives_addr.clone(),
        &ExecuteMsg::Expire {},
        &[],
    );
    assert!(result.is_ok());
    let result = context.app.execute_contract(
        Addr::unchecked(ADMIN),
        dao_voting_incentives_cw20_addr.clone(),
        &ExecuteMsg::Expire {},
        &[],
    );
    assert!(result.is_ok());

    // Ensure expire errors if already expired
    let result = context.app.execute_contract(
        Addr::unchecked(ADMIN),
        dao_voting_incentives_addr.clone(),
        &ExecuteMsg::Expire {},
        &[],
    );
    assert!(result.is_err());

    // Random person cannot claim
    let result = context.app.execute_contract(
        Addr::unchecked("random"),
        dao_voting_incentives_addr.clone(),
        &ExecuteMsg::Claim {},
        &[],
    );
    assert!(result.is_err());

    // Check rewards
    let rewards: RewardResponse = context
        .app
        .wrap()
        .query_wasm_smart(
            dao_voting_incentives_addr.clone(),
            &QueryMsg::Rewards {
                address: ADDR1.to_string(),
            },
        )
        .unwrap();
    assert_eq!(
        rewards,
        RewardResponse {
            denom: CheckedDenom::Native(DENOM.to_string()),
            amount: Uint128::new(500),
            is_claimable: true,
        }
    );

    // User claims rewards
    let result = context.app.execute_contract(
        Addr::unchecked(ADDR1),
        dao_voting_incentives_addr.clone(),
        &ExecuteMsg::Claim {},
        &[],
    );
    assert!(result.is_ok());

    // User balance has increased by 500, because 1000 reward with 2 voters during the period
    let balance = context
        .app
        .wrap()
        .query_balance(Addr::unchecked(ADDR1), DENOM)
        .unwrap();
    assert_eq!(balance.amount, Uint128::new(500));

    // User cannot claim again
    let result = context.app.execute_contract(
        Addr::unchecked(ADDR1),
        dao_voting_incentives_addr.clone(),
        &ExecuteMsg::Claim {},
        &[],
    );
    assert!(result.is_err());

    // User claims rewards cw20
    let result = context.app.execute_contract(
        Addr::unchecked(ADDR1),
        dao_voting_incentives_cw20_addr.clone(),
        &ExecuteMsg::Claim {},
        &[],
    );
    assert!(result.is_ok());

    // User balance has increased by 500, because 1000 reward with 2 voters during the period
    let balance_response: BalanceResponse = context
        .app
        .wrap()
        .query_wasm_smart(
            context.cw20_addr,
            &Cw20QueryMsg::Balance {
                address: ADDR1.to_string(),
            },
        )
        .unwrap();
    assert_eq!(balance_response.balance, Uint128::new(500));
}

#[test]
pub fn test_expire_sends_funds_to_owner() {
    let mut context = get_context();

    // Create the voting incentives contract for native
    // The expiration is 10 blocks from start (12345 height)
    let dao_voting_incentives_addr = context
        .app
        .instantiate_contract(
            context.dao_voting_incentives_code_id,
            Addr::unchecked(ADMIN),
            &InstantiateMsg {
                owner: context.dao_addr.to_string(),
                denom: UncheckedDenom::Native(DENOM.to_string()),
                expiration: Expiration::AtHeight(12355u64),
            },
            &[],
            "dao_voting_incentives".to_string(),
            None,
        )
        .unwrap();

    // Fund the incentives contracts for 1000
    context
        .app
        .send_tokens(
            Addr::unchecked(ADMIN),
            dao_voting_incentives_addr.clone(),
            &[Coin {
                denom: DENOM.to_string(),
                amount: Uint128::new(1000),
            }],
        )
        .unwrap();

    // Blocks have passed the voting incentives' expirations
    context.app.update_block(|x| x.height += 100);

    // Expire the vote hooks
    // No votes were received during the period, so the funds should be sent to the owner on expiration
    let result = context.app.execute_contract(
        Addr::unchecked(ADMIN),
        dao_voting_incentives_addr.clone(),
        &ExecuteMsg::Expire {},
        &[],
    );
    assert!(result.is_ok());

    // Ensure funds were sent to the DAO
    let balance = context
        .app
        .wrap()
        .query_balance(context.dao_addr, DENOM)
        .unwrap();
    assert_eq!(balance.amount, Uint128::new(1000));
}

#[test]
pub fn test_migrate_update_version() {
    let mut deps = mock_dependencies();
    cw2::set_contract_version(&mut deps.storage, "my-contract", "old-version").unwrap();
    migrate(deps.as_mut(), mock_env(), MigrateMsg::FromCompatible {}).unwrap();
    let version = cw2::get_contract_version(&deps.storage).unwrap();
    assert_eq!(version.version, CONTRACT_VERSION);
    assert_eq!(version.contract, CONTRACT_NAME);
}
