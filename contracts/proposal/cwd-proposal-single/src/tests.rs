use std::u128;

use cosmwasm_std::{
    coins,
    testing::{mock_dependencies, mock_env},
    to_binary, Addr, Coin, CosmosMsg, Decimal, Empty, Order, Timestamp, Uint128, WasmMsg,
};
use cw20::Cw20Coin;
use cwd_voting_cw20_staked::msg::ActiveThreshold;

use cw_multi_test::{next_block, App, BankSudo, Contract, ContractWrapper, Executor, SudoMsg};
use cw_pre_propose_base_proposal_single as cppbps;
use cw_storage_plus::{Item, Map};
use cw_utils::Duration;
use cw_utils::Expiration;
use cwd_core::state::ProposalModule;
use cwd_interface::{Admin, ModuleInstantiateInfo};

use cwd_hooks::HooksResponse;

use cw_denom::{CheckedDenom, UncheckedDenom};

use cwd_voting::{
    deposit::{CheckedDepositInfo, DepositRefundPolicy, DepositToken, UncheckedDepositInfo},
    pre_propose::{PreProposeInfo, ProposalCreationPolicy},
    status::Status,
    threshold::{PercentageThreshold, Threshold},
    voting::{Vote, Votes},
};
use testing::{ShouldExecute, TestSingleChoiceVote};

use crate::{
    contract::{migrate, CONTRACT_NAME, CONTRACT_VERSION},
    msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg},
    proposal::SingleChoiceProposal,
    query::{ProposalListResponse, ProposalResponse, VoteInfo, VoteResponse},
    state::Config,
    ContractError,
};

const CREATOR_ADDR: &str = "creator";

#[cw_serde]
struct V1Proposal {
    pub title: String,
    pub description: String,
    pub proposer: Addr,
    pub start_height: u64,
    pub min_voting_period: Option<Expiration>,
    pub expiration: Expiration,
    pub threshold: Threshold,
    pub total_power: Uint128,
    pub msgs: Vec<CosmosMsg<Empty>>,
    pub status: Status,
    pub votes: Votes,
    pub allow_revoting: bool,
    pub deposit_info: Option<CheckedDepositInfo>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct V1Config {
    pub threshold: Threshold,
    pub max_voting_period: Duration,
    pub min_voting_period: Option<Duration>,
    pub only_members_execute: bool,
    pub allow_revoting: bool,
    pub dao: Addr,
    pub deposit_info: Option<CheckedDepositInfo>,
}

#[test]
fn test_migrate_mock() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let current_block = &env.block;
    let max_voting_period = cw_utils::Duration::Height(6);

    let threshold = Threshold::AbsolutePercentage {
        percentage: PercentageThreshold::Majority {},
    };

    // Write to storage in old data format
    let v1_map: Map<u64, V1Proposal> = Map::new("proposals");
    let v1_proposal = V1Proposal {
        title: "A simple text proposal".to_string(),
        description: "This is a simple text proposal".to_string(),
        proposer: Addr::unchecked(CREATOR_ADDR),
        start_height: env.block.height,
        expiration: max_voting_period.after(current_block),
        min_voting_period: None,
        threshold: threshold.clone(),
        allow_revoting: false,
        total_power: Uint128::new(100_000_000),
        msgs: vec![],
        status: Status::Open,
        votes: Votes::zero(),
        deposit_info: None,
    };
    v1_map.save(&mut deps.storage, 0, &v1_proposal).unwrap();

    let v1_item: Item<V1Config> = Item::new("config");
    let v1_config = V1Config {
        threshold: threshold.clone(),
        max_voting_period,
        min_voting_period: None,
        only_members_execute: true,
        allow_revoting: false,
        dao: Addr::unchecked("simple happy desert"),
        deposit_info: None,
    };
    v1_item.save(&mut deps.storage, &v1_config).unwrap();

    let msg = MigrateMsg::FromV1 {
        close_proposal_on_execution_failure: true,
    };
    migrate(deps.as_mut(), env.clone(), msg).unwrap();

    // Verify migration.
    let new_map: Map<u64, SingleChoiceProposal> = Map::new("proposals_v2");
    let proposals: Vec<(u64, SingleChoiceProposal)> = new_map
        .range(&deps.storage, None, None, Order::Ascending)
        .collect::<Result<Vec<(u64, SingleChoiceProposal)>, _>>()
        .unwrap();

    let migrated_proposal = &proposals[0];
    assert_eq!(migrated_proposal.0, 0);

    let new_item: Item<Config> = Item::new("config_v2");
    let migrated_config = new_item.load(&deps.storage).unwrap();
    // assert_eq!(
    //     migrated_config,
    //     Config {
    //         threshold,
    //         max_voting_period,
    //         min_voting_period: None,
    //         only_members_execute: true,
    //         allow_revoting: false,
    //         dao: Addr::unchecked("simple happy desert"),
    //         deposit_info: None,
    //         close_proposal_on_execution_failure: true,
    //         open_proposal_submission: false,
    //     }
    // );
    todo!("(zeke) hmmmmmmmmm")
}

#[test]
fn test_close_failed_proposal() {
    let mut app = App::default();
    let govmod_id = app.store_code(proposal_contract());

    let threshold = Threshold::AbsolutePercentage {
        percentage: PercentageThreshold::Majority {},
    };
    let max_voting_period = cw_utils::Duration::Height(6);
    let instantiate = InstantiateMsg {
        threshold,
        max_voting_period,
        min_voting_period: None,
        only_members_execute: false,
        allow_revoting: false,
        pre_propose_info: get_pre_propose_info(&mut app, None, false),
        close_proposal_on_execution_failure: true,
    };

    let governance_addr =
        instantiate_with_staking_active_threshold(&mut app, govmod_id, instantiate, None, None);
    let governance_modules: Vec<ProposalModule> = app
        .wrap()
        .query_wasm_smart(
            governance_addr,
            &cwd_core::msg::QueryMsg::ProposalModules {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(governance_modules.len(), 1);
    let govmod_single = governance_modules.into_iter().next().unwrap().address;

    let govmod_config: Config = app
        .wrap()
        .query_wasm_smart(govmod_single.clone(), &QueryMsg::Config {})
        .unwrap();
    let dao = govmod_config.dao;
    let voting_module: Addr = app
        .wrap()
        .query_wasm_smart(dao, &cwd_core::msg::QueryMsg::VotingModule {})
        .unwrap();
    let staking_contract: Addr = app
        .wrap()
        .query_wasm_smart(
            voting_module.clone(),
            &cwd_voting_cw20_staked::msg::QueryMsg::StakingContract {},
        )
        .unwrap();
    let token_contract: Addr = app
        .wrap()
        .query_wasm_smart(
            voting_module,
            &cwd_interface::voting::Query::TokenContract {},
        )
        .unwrap();

    // Stake some tokens so we can propose
    let msg = cw20::Cw20ExecuteMsg::Send {
        contract: staking_contract.to_string(),
        amount: Uint128::new(2000),
        msg: to_binary(&cw20_stake::msg::ReceiveMsg::Stake {}).unwrap(),
    };
    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        token_contract.clone(),
        &msg,
        &[],
    )
    .unwrap();
    app.update_block(next_block);

    let msg = cw20::Cw20ExecuteMsg::Burn {
        amount: Uint128::new(2000),
    };
    let binary_msg = to_binary(&msg).unwrap();

    // Overburn tokens
    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        govmod_single.clone(),
        &ExecuteMsg::Propose {
            title: "A simple burn tokens proposal".to_string(),
            description: "Burning more tokens, than dao treasury have".to_string(),
            msgs: vec![WasmMsg::Execute {
                contract_addr: token_contract.to_string(),
                msg: binary_msg.clone(),
                funds: vec![],
            }
            .into()],
        },
        &[],
    )
    .unwrap();

    // Vote on proposal
    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        govmod_single.clone(),
        &ExecuteMsg::Vote {
            proposal_id: 1,
            vote: Vote::Yes,
        },
        &[],
    )
    .unwrap();

    let timestamp = Timestamp::from_seconds(300_000_000);
    app.update_block(|block| block.time = timestamp);

    // Execute proposal
    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        govmod_single.clone(),
        &ExecuteMsg::Execute { proposal_id: 1 },
        &[],
    )
    .unwrap();

    let failed: ProposalResponse = app
        .wrap()
        .query_wasm_smart(
            govmod_single.clone(),
            &QueryMsg::Proposal { proposal_id: 1 },
        )
        .unwrap();
    assert_eq!(failed.proposal.status, Status::ExecutionFailed);

    // With disabled feature
    // Disable feature first
    {
        let original: Config = app
            .wrap()
            .query_wasm_smart(govmod_single.clone(), &QueryMsg::Config {})
            .unwrap();

        let pre_propose_info = get_pre_propose_info(&mut app, None, false);
        app.execute_contract(
            Addr::unchecked(CREATOR_ADDR),
            govmod_single.clone(),
            &ExecuteMsg::Propose {
                title: "Disable closing failed proposals".to_string(),
                description: "We want to re-execute failed proposals".to_string(),
                msgs: vec![WasmMsg::Execute {
                    contract_addr: govmod_single.to_string(),
                    msg: to_binary(&ExecuteMsg::UpdateConfig {
                        threshold: original.threshold,
                        max_voting_period: original.max_voting_period,
                        min_voting_period: original.min_voting_period,
                        only_members_execute: original.only_members_execute,
                        allow_revoting: original.allow_revoting,
                        dao: original.dao.to_string(),
                        pre_propose_info,
                        close_proposal_on_execution_failure: false,
                    })
                    .unwrap(),
                    funds: vec![],
                }
                .into()],
            },
            &[],
        )
        .unwrap();

        // Vote on proposal
        app.execute_contract(
            Addr::unchecked(CREATOR_ADDR),
            govmod_single.clone(),
            &ExecuteMsg::Vote {
                proposal_id: 2,
                vote: Vote::Yes,
            },
            &[],
        )
        .unwrap();

        // Execute proposal
        app.execute_contract(
            Addr::unchecked(CREATOR_ADDR),
            govmod_single.clone(),
            &ExecuteMsg::Execute { proposal_id: 2 },
            &[],
        )
        .unwrap();
    }

    // Overburn tokens (again), this time without reverting
    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        govmod_single.clone(),
        &ExecuteMsg::Propose {
            title: "A simple burn tokens proposal".to_string(),
            description: "Burning more tokens, than dao treasury have".to_string(),
            msgs: vec![WasmMsg::Execute {
                contract_addr: token_contract.to_string(),
                msg: binary_msg,
                funds: vec![],
            }
            .into()],
        },
        &[],
    )
    .unwrap();

    // Vote on proposal
    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        govmod_single.clone(),
        &ExecuteMsg::Vote {
            proposal_id: 3,
            vote: Vote::Yes,
        },
        &[],
    )
    .unwrap();

    // Execute proposal
    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        govmod_single.clone(),
        &ExecuteMsg::Execute { proposal_id: 3 },
        &[],
    )
    .expect_err("Should be sub overflow");

    // Status should still be passed
    let updated: ProposalResponse = app
        .wrap()
        .query_wasm_smart(govmod_single, &QueryMsg::Proposal { proposal_id: 3 })
        .unwrap();

    // not reverted
    assert_eq!(updated.proposal.status, Status::Passed);
}

#[test]
fn test_no_double_refund_on_execute_fail_and_close() {
    let mut app = App::default();
    let proposal_module_id = app.store_code(proposal_contract());

    let threshold = Threshold::AbsolutePercentage {
        percentage: PercentageThreshold::Majority {},
    };
    let max_voting_period = cw_utils::Duration::Height(6);
    let instantiate = InstantiateMsg {
        threshold,
        max_voting_period,
        min_voting_period: None,
        only_members_execute: false,
        allow_revoting: false,
        pre_propose_info: get_pre_propose_info(
            &mut app,
            Some(UncheckedDepositInfo {
                denom: DepositToken::VotingModuleToken {},
                amount: Uint128::new(1),
                // Important to set to always here as we want to be sure
                // that we don't get a second refund on close. Refunds on
                // close only happen if Deposity Refund Policy is "Always".
                refund_policy: DepositRefundPolicy::Always,
            }),
            false,
        ),
        close_proposal_on_execution_failure: true,
    };

    let core_addr = instantiate_with_staking_active_threshold(
        &mut app,
        proposal_module_id,
        instantiate,
        Some(vec![Cw20Coin {
            address: CREATOR_ADDR.to_string(),
            // One token for sending to the DAO treasury, one token
            // for staking, one token for paying the proposal deposit.
            amount: Uint128::new(3),
        }]),
        None,
    );
    let proposal_modules: Vec<ProposalModule> = app
        .wrap()
        .query_wasm_smart(
            core_addr,
            &cwd_core::msg::QueryMsg::ProposalModules {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(proposal_modules.len(), 1);
    let proposal_single = proposal_modules.into_iter().next().unwrap().address;

    let proposal_config: Config = app
        .wrap()
        .query_wasm_smart(proposal_single.clone(), &QueryMsg::Config {})
        .unwrap();
    let dao = proposal_config.dao;
    let voting_module: Addr = app
        .wrap()
        .query_wasm_smart(dao, &cwd_core::msg::QueryMsg::VotingModule {})
        .unwrap();
    let staking_contract: Addr = app
        .wrap()
        .query_wasm_smart(
            voting_module.clone(),
            &cwd_voting_cw20_staked::msg::QueryMsg::StakingContract {},
        )
        .unwrap();
    let token_contract: Addr = app
        .wrap()
        .query_wasm_smart(
            voting_module,
            &cwd_interface::voting::Query::TokenContract {},
        )
        .unwrap();

    // Stake a token so we can propose.
    let msg = cw20::Cw20ExecuteMsg::Send {
        contract: staking_contract.to_string(),
        amount: Uint128::new(1),
        msg: to_binary(&cw20_stake::msg::ReceiveMsg::Stake {}).unwrap(),
    };
    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        token_contract.clone(),
        &msg,
        &[],
    )
    .unwrap();
    app.update_block(next_block);

    // Send some tokens to the proposal module so it has the ability
    // to double refund if the code is buggy.
    let msg = cw20::Cw20ExecuteMsg::Transfer {
        recipient: proposal_single.to_string(),
        amount: Uint128::new(1),
    };
    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        token_contract.clone(),
        &msg,
        &[],
    )
    .unwrap();

    let msg = cw20::Cw20ExecuteMsg::Burn {
        amount: Uint128::new(2000),
    };
    let binary_msg = to_binary(&msg).unwrap();

    // Increase allowance to pay the proposal deposit.
    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        token_contract.clone(),
        &cw20_base::msg::ExecuteMsg::IncreaseAllowance {
            spender: proposal_single.to_string(),
            amount: Uint128::new(1),
            expires: None,
        },
        &[],
    )
    .unwrap();

    // proposal to overburn tokens
    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        proposal_single.clone(),
        &ExecuteMsg::Propose {
            title: "A simple burn tokens proposal".to_string(),
            description: "Burning more tokens, than dao treasury have".to_string(),
            msgs: vec![WasmMsg::Execute {
                contract_addr: token_contract.to_string(),
                msg: binary_msg,
                funds: vec![],
            }
            .into()],
        },
        &[],
    )
    .unwrap();

    // Vote on proposal
    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        proposal_single.clone(),
        &ExecuteMsg::Vote {
            proposal_id: 1,
            vote: Vote::Yes,
        },
        &[],
    )
    .unwrap();

    let timestamp = Timestamp::from_seconds(300_000_000);
    app.update_block(|block| block.time = timestamp);

    // Execute proposal
    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        proposal_single.clone(),
        &ExecuteMsg::Execute { proposal_id: 1 },
        &[],
    )
    .unwrap();

    let failed: ProposalResponse = app
        .wrap()
        .query_wasm_smart(
            proposal_single.clone(),
            &QueryMsg::Proposal { proposal_id: 1 },
        )
        .unwrap();
    assert_eq!(failed.proposal.status, Status::ExecutionFailed);

    // Check that our deposit has been refunded.
    let balance: cw20::BalanceResponse = app
        .wrap()
        .query_wasm_smart(
            token_contract.to_string(),
            &cw20::Cw20QueryMsg::Balance {
                address: CREATOR_ADDR.to_string(),
            },
        )
        .unwrap();

    assert_eq!(balance.balance, Uint128::new(1));

    // Close the proposal - this should fail as it was executed.
    let err: ContractError = app
        .execute_contract(
            Addr::unchecked(CREATOR_ADDR),
            proposal_single,
            &ExecuteMsg::Close { proposal_id: 1 },
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();

    assert!(matches!(err, ContractError::WrongCloseStatus {}));

    // Check that our deposit was not refunded a second time on close.
    let balance: cw20::BalanceResponse = app
        .wrap()
        .query_wasm_smart(
            token_contract.to_string(),
            &cw20::Cw20QueryMsg::Balance {
                address: CREATOR_ADDR.to_string(),
            },
        )
        .unwrap();

    assert_eq!(balance.balance, Uint128::new(1));
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
