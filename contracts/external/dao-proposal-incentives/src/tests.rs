use std::vec;

use cosmwasm_std::{
    testing::{mock_dependencies, mock_env},
    to_json_binary, Addr, Binary, Coin, CosmosMsg, Empty, Uint128, WasmMsg,
};

use cw20::Cw20Coin;
use cw_denom::{CheckedDenom, UncheckedDenom};
use cw_multi_test::{
    error::AnyResult, App, AppBuilder, AppResponse, Contract, ContractWrapper, Executor,
};
use cw_ownable::Ownership;
use dao_testing::{
    contracts::{dao_proposal_incentives_contract, proposal_single_contract},
    helpers::instantiate_with_cw4_groups_governance,
};
use dao_voting::{proposal::SingleChoiceProposeMsg, threshold::Threshold};

use crate::{
    contract::{migrate, CONTRACT_NAME, CONTRACT_VERSION},
    msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, ProposalIncentivesUnchecked, QueryMsg},
    state::ProposalIncentives,
};

const ADMIN: &str = "admin";
const ADDR1: &str = "addr1";
const DENOM: &str = "juno";

struct Context {
    app: App,
    cw20_addr: Addr,
    proposal_single_addr: Addr,
    dao_addr: Addr,
    dao_proposal_incentives_code_id: u64,
}

fn cw20_base_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cw20_base::contract::execute,
        cw20_base::contract::instantiate,
        cw20_base::contract::query,
    );
    Box::new(contract)
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
        Some(vec![Cw20Coin {
            address: ADDR1.to_string(),
            amount: Uint128::one(),
        }]),
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

    // Set up dao proposal incentives code id
    let dao_proposal_incentives_code_id = app.store_code(dao_proposal_incentives_contract());

    Context {
        app,
        cw20_addr,
        dao_addr,
        dao_proposal_incentives_code_id,
        proposal_single_addr,
    }
}

fn vote_yes_on_proposal(context: &mut Context, proposal_id: u64) -> AnyResult<AppResponse> {
    context.app.execute_contract(
        Addr::unchecked(ADDR1),
        context.proposal_single_addr.clone(),
        &dao_proposal_single::msg::ExecuteMsg::Vote {
            proposal_id,
            vote: dao_voting::voting::Vote::Yes,
            rationale: None,
        },
        &[],
    )
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
pub fn test_setup_native() {
    let mut context = get_context();

    // Cannot instantiate with 0 due
    let result = context.app.instantiate_contract(
        context.dao_proposal_incentives_code_id,
        Addr::unchecked(ADMIN),
        &InstantiateMsg {
            owner: ADMIN.to_string(),
            proposal_incentives: ProposalIncentivesUnchecked {
                rewards_per_proposal: Uint128::zero(),
                denom: UncheckedDenom::Native(DENOM.to_string()),
            },
        },
        &[],
        "dao_proposal_incentives".to_string(),
        None,
    );
    assert!(result.is_err());

    // Can instantiate with some due
    let result = context.app.instantiate_contract(
        context.dao_proposal_incentives_code_id,
        Addr::unchecked(ADMIN),
        &InstantiateMsg {
            owner: ADMIN.to_string(),
            proposal_incentives: ProposalIncentivesUnchecked {
                rewards_per_proposal: Uint128::new(1000),
                denom: UncheckedDenom::Native(DENOM.to_string()),
            },
        },
        &[],
        "dao_proposal_incentives".to_string(),
        None,
    );
    assert!(result.is_ok());
    let dao_proposal_incentives_addr = result.unwrap();

    // Ensure owner was set on init
    let ownership: Ownership<String> = context
        .app
        .wrap()
        .query_wasm_smart(
            dao_proposal_incentives_addr.clone(),
            &QueryMsg::Ownership {},
        )
        .unwrap();

    assert_eq!(ownership.owner, Some(ADMIN.to_string()));

    // Ensure proposal incentives was set
    let proposal_incentives: ProposalIncentives = context
        .app
        .wrap()
        .query_wasm_smart(
            dao_proposal_incentives_addr.clone(),
            &QueryMsg::ProposalIncentives { height: None },
        )
        .unwrap();
    assert_eq!(
        proposal_incentives,
        ProposalIncentives {
            rewards_per_proposal: Uint128::new(1000),
            denom: CheckedDenom::Native(DENOM.to_string())
        }
    );

    // Cannot update rewards to zero
    let result = context.app.execute_contract(
        Addr::unchecked(ADMIN),
        dao_proposal_incentives_addr.clone(),
        &ExecuteMsg::UpdateProposalIncentives {
            proposal_incentives: ProposalIncentivesUnchecked {
                rewards_per_proposal: Uint128::zero(),
                denom: UncheckedDenom::Native(DENOM.to_string()),
            },
        },
        &[],
    );
    assert!(result.is_err());

    // Cannot update unauthorized
    let result = context.app.execute_contract(
        Addr::unchecked(ADDR1),
        dao_proposal_incentives_addr.clone(),
        &ExecuteMsg::UpdateProposalIncentives {
            proposal_incentives: ProposalIncentivesUnchecked {
                rewards_per_proposal: Uint128::one(),
                denom: UncheckedDenom::Native(DENOM.to_string()),
            },
        },
        &[],
    );
    assert!(result.is_err());

    // Can update rewards
    let result = context.app.execute_contract(
        Addr::unchecked(ADMIN),
        dao_proposal_incentives_addr.clone(),
        &ExecuteMsg::UpdateProposalIncentives {
            proposal_incentives: ProposalIncentivesUnchecked {
                rewards_per_proposal: Uint128::one(),
                denom: UncheckedDenom::Cw20(context.cw20_addr.to_string()),
            },
        },
        &[],
    );
    assert!(result.is_ok());

    // Ensure proposal incentives was updated
    let proposal_incentives: ProposalIncentives = context
        .app
        .wrap()
        .query_wasm_smart(
            dao_proposal_incentives_addr.clone(),
            &QueryMsg::ProposalIncentives { height: None },
        )
        .unwrap();
    assert_eq!(
        proposal_incentives,
        ProposalIncentives {
            rewards_per_proposal: Uint128::one(),
            denom: CheckedDenom::Cw20(context.cw20_addr.clone()),
        }
    );
}

#[test]
pub fn test_hook() {
    let mut context = get_context();

    // Create the proposal incentives contract
    let dao_proposal_incentives_addr = context
        .app
        .instantiate_contract(
            context.dao_proposal_incentives_code_id,
            Addr::unchecked(ADMIN),
            &InstantiateMsg {
                owner: context.dao_addr.to_string(),
                proposal_incentives: ProposalIncentivesUnchecked {
                    rewards_per_proposal: Uint128::new(1000),
                    denom: UncheckedDenom::Native(DENOM.to_string()),
                },
            },
            &[],
            "dao_proposal_incentives".to_string(),
            None,
        )
        .unwrap();
    context.app.update_block(|x| x.height += 10);

    // Execute fails - unauthorized
    let result = context.app.execute_contract(
        Addr::unchecked(ADMIN),
        dao_proposal_incentives_addr.clone(),
        &ExecuteMsg::ProposalHook(
            dao_hooks::proposal::ProposalHookMsg::ProposalStatusChanged {
                id: 1u64,
                old_status: "open".to_string(),
                new_status: "passed".to_string(),
            },
        ),
        &[],
    );
    assert!(result.is_err());

    // Fund the incentives contract for 1 reward
    context
        .app
        .send_tokens(
            Addr::unchecked(ADMIN),
            dao_proposal_incentives_addr.clone(),
            &[Coin {
                denom: DENOM.to_string(),
                amount: Uint128::new(1000),
            }],
        )
        .unwrap();

    // Fund the incentives contract with cw20 as well to show cw20 support
    let result = context.app.execute_contract(
        Addr::unchecked(ADMIN),
        context.cw20_addr.clone(),
        &cw20::Cw20ExecuteMsg::Send {
            contract: dao_proposal_incentives_addr.to_string(),
            amount: Uint128::new(1000),
            msg: Binary::default(),
        },
        &[],
    );
    assert!(result.is_ok());

    // Propose adding a hook
    context
        .app
        .execute_contract(
            Addr::unchecked(ADDR1),
            context.proposal_single_addr.clone(),
            &dao_proposal_single::msg::ExecuteMsg::Propose(SingleChoiceProposeMsg {
                title: "Add proposal hook".to_string(),
                description: "Adding a proposal hook to test the dao_proposal_incentives contract"
                    .to_string(),
                msgs: vec![CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: context.proposal_single_addr.to_string(),
                    msg: to_json_binary(&dao_proposal_single::msg::ExecuteMsg::AddProposalHook {
                        address: dao_proposal_incentives_addr.to_string(),
                    })
                    .unwrap(),
                    funds: vec![],
                })],
                proposer: None,
            }),
            &[],
        )
        .unwrap();

    // Vote and execute the proposal to add the proposal hook
    vote_yes_on_proposal(&mut context, 1u64).unwrap();
    execute_proposal(&mut context, 1u64);

    // Query for the newly-established hook
    let result: cw_hooks::HooksResponse = context
        .app
        .wrap()
        .query_wasm_smart(
            context.proposal_single_addr.clone(),
            &dao_proposal_single::msg::QueryMsg::ProposalHooks {},
        )
        .unwrap();
    assert!(result
        .hooks
        .contains(&dao_proposal_incentives_addr.to_string()));

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

    // Assert that the proposal hook's execution has sent funds to proposer
    let result = vote_yes_on_proposal(&mut context, 2u64);
    assert!(result.is_ok());
    let balance = context.app.wrap().query_balance(ADDR1, DENOM).unwrap();
    assert_eq!(
        balance,
        Coin {
            denom: DENOM.to_string(),
            amount: Uint128::new(1000)
        }
    );

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

    // Assert that the proposal hook's failure still allows completion
    // The hook is attempting to send funds when it has run out of funds
    let result = vote_yes_on_proposal(&mut context, 3u64);
    assert!(result.is_ok());
    assert!(result.unwrap().events.iter().any(|x| x
        .attributes
        .iter()
        .any(|y| y.key == "removed_proposal_hook"
            && y.value == format!("{0}:0", dao_proposal_incentives_addr))));
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
