use cosmwasm_std::{to_json_binary, Addr, Decimal, Empty, Uint128};
use cw20::Cw20Coin;
use cw_multi_test::{next_block, App, Contract, ContractWrapper, Executor};
use dao_interface::{
    msg::QueryMsg as DaoQueryMsg,
    state::{Admin, ModuleInstantiateInfo},
    voting::Query as VotingQueryMsg,
};
use dao_testing::contracts::{
    cw20_stake_contract, cw20_staked_balances_voting_contract, dao_dao_contract,
    pre_propose_single_contract, proposal_single_contract,
};
use dao_voting::{
    deposit::{DepositRefundPolicy, DepositToken, UncheckedDepositInfo, VotingModuleTokenType},
    pre_propose::PreProposeInfo,
    threshold::PercentageThreshold,
    threshold::Threshold,
};

use crate::{
    msg::{ExecuteMsg, InstantiateMsg},
    ContractError,
};

fn dao_cw20_transfer_rules_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    );
    Box::new(contract)
}

fn cw20_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cw20_token::contract::execute,
        cw20_token::contract::instantiate,
        cw20_token::contract::query,
    );
    Box::new(contract)
}

const ADDR1: &str = "addr1";
const ADDR2: &str = "addr2";
const ADDR3: &str = "addr3";
const ALLOWED: &str = "allowed";
const OWNER: &str = "owner";
const RANDOM: &str = "random";

fn setup_cw20_dao(app: &mut App) -> (Addr, Addr, Addr, Addr) {
    let cw20_code_id = app.store_code(cw20_contract());
    let dao_dao_core_id = app.store_code(dao_dao_contract());
    let prop_single_id = app.store_code(proposal_single_contract());
    let pre_propose_single_id = app.store_code(pre_propose_single_contract());
    let cw20_stake_id = app.store_code(cw20_stake_contract());
    let cw20_voting_code_id = app.store_code(cw20_staked_balances_voting_contract());

    let initial_balances = vec![
        Cw20Coin {
            address: ADDR1.to_string(),
            amount: Uint128::new(100_000_000),
        },
        Cw20Coin {
            address: ADDR2.to_string(),
            amount: Uint128::new(100_000_000),
        },
        Cw20Coin {
            address: ADDR3.to_string(),
            amount: Uint128::new(100_000_000),
        },
    ];

    let msg = dao_interface::msg::InstantiateMsg {
        admin: None,
        name: "DAO DAO".to_string(),
        description: "A DAO that makes DAO tooling".to_string(),
        image_url: Some("https://zmedley.com/raw_logo.png".to_string()),
        dao_uri: None,
        automatically_add_cw20s: false,
        automatically_add_cw721s: false,
        voting_module_instantiate_info: ModuleInstantiateInfo {
            code_id: cw20_voting_code_id,
            msg: to_json_binary(&dao_voting_cw20_staked::msg::InstantiateMsg {
                token_info: dao_voting_cw20_staked::msg::TokenInfo::New {
                    code_id: cw20_code_id,
                    label: "DAO DAO Gov token".to_string(),
                    name: "DAO".to_string(),
                    symbol: "DAO".to_string(),
                    decimals: 6,
                    initial_balances: initial_balances.clone(),
                    marketing: None,
                    staking_code_id: cw20_stake_id,
                    unstaking_duration: Some(cw_utils::Duration::Time(1209600)),
                    initial_dao_balance: Some(Uint128::new(1000000000)),
                },
                active_threshold: None,
            })
            .unwrap(),
            funds: vec![],
            admin: Some(Admin::CoreModule {}),
            label: "DAO DAO Voting Module".to_string(),
        },
        proposal_modules_instantiate_info: vec![ModuleInstantiateInfo {
            code_id: prop_single_id,
            msg: to_json_binary(&dao_proposal_single::msg::InstantiateMsg {
                min_voting_period: None,
                threshold: Threshold::ThresholdQuorum {
                    threshold: PercentageThreshold::Majority {},
                    quorum: PercentageThreshold::Percent(Decimal::percent(10)),
                },
                max_voting_period: cw_utils::Duration::Time(432000),
                allow_revoting: false,
                only_members_execute: true,
                pre_propose_info: PreProposeInfo::ModuleMayPropose {
                    info: ModuleInstantiateInfo {
                        code_id: pre_propose_single_id,
                        msg: to_json_binary(&dao_pre_propose_single::InstantiateMsg {
                            deposit_info: Some(UncheckedDepositInfo {
                                denom: DepositToken::VotingModuleToken {
                                    token_type: VotingModuleTokenType::Cw20,
                                },
                                amount: Uint128::new(1000000000),
                                refund_policy: DepositRefundPolicy::OnlyPassed,
                            }),
                            open_proposal_submission: false,
                            extension: Empty::default(),
                        })
                        .unwrap(),
                        admin: Some(Admin::CoreModule {}),
                        funds: vec![],
                        label: "DAO DAO Pre-Propose Module".to_string(),
                    },
                },
                close_proposal_on_execution_failure: false,
                veto: None,
            })
            .unwrap(),
            admin: Some(Admin::CoreModule {}),
            funds: vec![],
            label: "DAO DAO Proposal Module".to_string(),
        }],
        initial_items: None,
    };

    // Instantiate DAO
    let dao_addr = app
        .instantiate_contract(
            dao_dao_core_id,
            Addr::unchecked(OWNER),
            &msg,
            &[],
            "Test DAO".to_string(),
            None,
        )
        .unwrap();

    // Get DAO voting module addr
    let voting_module_addr: Addr = app
        .wrap()
        .query_wasm_smart(dao_addr.clone(), &DaoQueryMsg::VotingModule {})
        .unwrap();

    // Get staking contract addr
    let staking_addr: Addr = app
        .wrap()
        .query_wasm_smart(
            voting_module_addr.clone(),
            &dao_voting_cw20_staked::msg::QueryMsg::StakingContract {},
        )
        .unwrap();

    // Get DAO cw20 token addr
    let cw20_addr: Addr = app
        .wrap()
        .query_wasm_smart(
            voting_module_addr.clone(),
            &VotingQueryMsg::TokenContract {},
        )
        .unwrap();

    // Stake tokens in the DAO
    for balance in initial_balances {
        app.execute_contract(
            Addr::unchecked(balance.address),
            cw20_addr.clone(),
            &cw20_token::msg::ExecuteMsg::Send {
                contract: staking_addr.to_string(),
                amount: Uint128::new(50_000),
                msg: to_json_binary(&cw20_stake::msg::ReceiveMsg::Stake {}).unwrap(),
            },
            &[],
        )
        .unwrap();
    }

    // Update the block so staked balances take effect
    app.update_block(next_block);

    (dao_addr, voting_module_addr, cw20_addr, staking_addr)
}

#[test]
pub fn test_transfer_rules() {
    let mut app = App::default();
    let (dao_addr, _, cw20_addr, _) = setup_cw20_dao(&mut app);
    let dao_cw20_transfer_rules_code_id = app.store_code(dao_cw20_transfer_rules_contract());

    let transfer_rules_addr = app
        .instantiate_contract(
            dao_cw20_transfer_rules_code_id,
            Addr::unchecked("owner"),
            &InstantiateMsg {
                dao: dao_addr.to_string(),
                allowlist: None,
            },
            &[],
            "dao-cw20-transfer-rules",
            None,
        )
        .unwrap();

    // Can't add hook if not owner / minter
    app.execute_contract(
        Addr::unchecked(RANDOM),
        cw20_addr.clone(),
        &dao_cw20::Cw20ExecuteMsg::AddHook {
            addr: transfer_rules_addr.to_string(),
        },
        &[],
    )
    .unwrap_err();

    // Add hook to the cw20 contract
    app.execute_contract(
        dao_addr.clone(),
        cw20_addr.clone(),
        &dao_cw20::Cw20ExecuteMsg::AddHook {
            addr: transfer_rules_addr.to_string(),
        },
        &[],
    )
    .unwrap();

    // Now that hook is added, members can't transfer to non-members
    let err: ContractError = app
        .execute_contract(
            Addr::unchecked(ADDR1),
            cw20_addr.clone(),
            &dao_cw20::Cw20ExecuteMsg::Transfer {
                recipient: Addr::unchecked(RANDOM).to_string(),
                amount: Uint128::new(100),
            },
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(err, ContractError::Unauthorized {});

    // Members can transfer to members
    app.execute_contract(
        Addr::unchecked(ADDR1),
        cw20_addr.clone(),
        &dao_cw20::Cw20ExecuteMsg::Transfer {
            recipient: Addr::unchecked(ADDR2).to_string(),
            amount: Uint128::new(100),
        },
        &[],
    )
    .unwrap();

    // Non-owner can't update allowlist
    let err: ContractError = app
        .execute_contract(
            Addr::unchecked(ADDR1),
            transfer_rules_addr.clone(),
            &ExecuteMsg::UpdateAllowlist {
                add: vec![Addr::unchecked(ALLOWED).to_string()],
                remove: vec![],
            },
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(
        err,
        ContractError::Ownable(cw_ownable::OwnershipError::NotOwner)
    );

    // Add a new address to the allowlist
    app.execute_contract(
        dao_addr.clone(),
        transfer_rules_addr.clone(),
        &ExecuteMsg::UpdateAllowlist {
            add: vec![Addr::unchecked(ALLOWED).to_string()],
            remove: vec![],
        },
        &[],
    )
    .unwrap();

    // Member can now transfer to new allowlisted address
    app.execute_contract(
        Addr::unchecked(ADDR1),
        cw20_addr.clone(),
        &dao_cw20::Cw20ExecuteMsg::Transfer {
            recipient: Addr::unchecked(ALLOWED).to_string(),
            amount: Uint128::new(100),
        },
        &[],
    )
    .unwrap();

    // DAO can transfer to non-members
    app.execute_contract(
        dao_addr.clone(),
        cw20_addr.clone(),
        &dao_cw20::Cw20ExecuteMsg::Transfer {
            recipient: Addr::unchecked(RANDOM).to_string(),
            amount: Uint128::new(100),
        },
        &[],
    )
    .unwrap();

    // Once added to the DAO, new members can transfer to DAO members
    app.execute_contract(
        Addr::unchecked(RANDOM),
        cw20_addr.clone(),
        &dao_cw20::Cw20ExecuteMsg::Transfer {
            recipient: Addr::unchecked(ADDR2).to_string(),
            amount: Uint128::new(100),
        },
        &[],
    )
    .unwrap();
}

#[test]
pub fn test_send_rules() {
    let mut app = App::default();
    let (dao_addr, _, cw20_addr, staking_addr) = setup_cw20_dao(&mut app);
    let dao_cw20_transfer_rules_code_id = app.store_code(dao_cw20_transfer_rules_contract());

    let transfer_rules_addr = app
        .instantiate_contract(
            dao_cw20_transfer_rules_code_id,
            Addr::unchecked("owner"),
            &InstantiateMsg {
                dao: dao_addr.to_string(),
                allowlist: None,
            },
            &[],
            "dao-cw20-transfer-rules",
            None,
        )
        .unwrap();

    // Can't add hook if not owner / minter
    app.execute_contract(
        Addr::unchecked(RANDOM),
        cw20_addr.clone(),
        &dao_cw20::Cw20ExecuteMsg::AddHook {
            addr: transfer_rules_addr.to_string(),
        },
        &[],
    )
    .unwrap_err();

    // Add hook to the cw20 contract
    app.execute_contract(
        dao_addr.clone(),
        cw20_addr.clone(),
        &dao_cw20::Cw20ExecuteMsg::AddHook {
            addr: transfer_rules_addr.to_string(),
        },
        &[],
    )
    .unwrap();

    // Now that hook is added, members can't send to non-members or non-allowlisted contracts
    // Including the staking contract.
    let err: ContractError = app
        .execute_contract(
            Addr::unchecked(ADDR1.to_string()),
            cw20_addr.clone(),
            &cw20_token::msg::ExecuteMsg::Send {
                contract: staking_addr.to_string(),
                amount: Uint128::new(50_000),
                msg: to_json_binary(&cw20_stake::msg::ReceiveMsg::Stake {}).unwrap(),
            },
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(err, ContractError::Unauthorized {});

    // Members can send to the DAO contract
    app.execute_contract(
        Addr::unchecked(ADDR1.to_string()),
        cw20_addr.clone(),
        &cw20_token::msg::ExecuteMsg::Send {
            contract: dao_addr.to_string(),
            amount: Uint128::new(5000),
            msg: to_json_binary(&Empty {}).unwrap(),
        },
        &[],
    )
    .unwrap();

    // Add staking contract to the allowlist
    app.execute_contract(
        dao_addr.clone(),
        transfer_rules_addr.clone(),
        &ExecuteMsg::UpdateAllowlist {
            add: vec![staking_addr.clone().to_string()],
            remove: vec![],
        },
        &[],
    )
    .unwrap();

    // Members can now send to the staking contract
    app.execute_contract(
        Addr::unchecked(ADDR1.to_string()),
        cw20_addr.clone(),
        &cw20_token::msg::ExecuteMsg::Send {
            contract: staking_addr.to_string(),
            amount: Uint128::new(50_000),
            msg: to_json_binary(&cw20_stake::msg::ReceiveMsg::Stake {}).unwrap(),
        },
        &[],
    )
    .unwrap();
}

#[test]
pub fn test_instantiate_invalid_dao_fails() {
    let mut app = App::default();
    let (_, _, cw20_addr, _) = setup_cw20_dao(&mut app);
    let dao_cw20_transfer_rules_code_id = app.store_code(dao_cw20_transfer_rules_contract());

    app.instantiate_contract(
        dao_cw20_transfer_rules_code_id,
        Addr::unchecked("owner"),
        &InstantiateMsg {
            dao: cw20_addr.to_string(),
            allowlist: None,
        },
        &[],
        "dao-cw20-transfer-rules",
        None,
    )
    .unwrap_err();
}
