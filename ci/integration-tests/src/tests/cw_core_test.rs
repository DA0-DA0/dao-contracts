use crate::helpers::chain::Chain;
use crate::helpers::helper::create_dao;
use assert_matches::assert_matches;
use cosm_orc::orchestrator::error::CosmwasmError::TxError;
use cosm_orc::orchestrator::error::ProcessError;
use cosmwasm_std::{to_binary, Addr, CosmosMsg, Decimal, Uint128};
use cw20_stake::msg::{StakedValueResponse, TotalValueResponse};

use cw_utils::Duration;
use cwd_core::query::{GetItemResponse, PauseInfoResponse};
use cwd_voting::{
    pre_propose::ProposalCreationPolicy, threshold::PercentageThreshold, threshold::Threshold,
};
use test_context::test_context;

// #### ExecuteMsg #####

// TODO: Add tests for all cw-core execute msgs

#[test_context(Chain)]
#[test]
#[ignore]
fn execute_execute_admin_msgs(chain: &mut Chain) {
    let user_addr = chain.users["user1"].account.address.clone();
    let user_key = chain.users["user1"].key.clone();

    // if you are not the admin, you cant execute admin msgs:
    let res = create_dao(
        chain,
        None,
        "exc_admin_msgs_create_dao",
        user_addr.clone(),
        &user_key,
    );
    let dao = res.unwrap();

    let res = chain.orc.execute(
        "cwd_core",
        "exc_admin_msgs_pause_dao_fail",
        &cwd_core::msg::ExecuteMsg::ExecuteAdminMsgs {
            msgs: vec![CosmosMsg::Wasm(cosmwasm_std::WasmMsg::Execute {
                contract_addr: dao.addr,
                msg: to_binary(&cwd_core::msg::ExecuteMsg::Pause {
                    duration: Duration::Time(100),
                })
                .unwrap(),
                funds: vec![],
            })],
        },
        &user_key,
        vec![],
    );

    assert_matches!(res.unwrap_err(), ProcessError::CosmwasmError(TxError(..)));

    let res = chain
        .orc
        .query("cwd_core", &cwd_core::msg::QueryMsg::PauseInfo {})
        .unwrap();
    let res: PauseInfoResponse = res.data().unwrap();

    assert_eq!(res, PauseInfoResponse::Unpaused {});

    // if you are the admin you can execute admin msgs:
    let res = create_dao(
        chain,
        Some(user_addr.clone()),
        "exc_admin_msgs_create_dao_with_admin",
        user_addr,
        &user_key,
    );
    let dao = res.unwrap();

    chain
        .orc
        .execute(
            "cwd_core",
            "exc_admin_msgs_pause_dao",
            &cwd_core::msg::ExecuteMsg::ExecuteAdminMsgs {
                msgs: vec![CosmosMsg::Wasm(cosmwasm_std::WasmMsg::Execute {
                    contract_addr: dao.addr,
                    msg: to_binary(&cwd_core::msg::ExecuteMsg::Pause {
                        duration: Duration::Height(100),
                    })
                    .unwrap(),
                    funds: vec![],
                })],
            },
            &user_key,
            vec![],
        )
        .unwrap();

    let res = chain
        .orc
        .query("cwd_core", &cwd_core::msg::QueryMsg::PauseInfo {})
        .unwrap();

    let res: PauseInfoResponse = res.data().unwrap();
    assert_ne!(res, PauseInfoResponse::Unpaused {});
}

#[test_context(Chain)]
#[test]
#[ignore]
fn execute_items(chain: &mut Chain) {
    let user_addr = chain.users["user1"].account.address.clone();
    let user_key = chain.users["user1"].key.clone();

    // add item:
    let res = create_dao(
        chain,
        Some(user_addr.clone()),
        "exc_items_create_dao",
        user_addr,
        &user_key,
    );

    let dao = res.unwrap();

    let res = chain
        .orc
        .query(
            "cwd_core",
            &cwd_core::msg::QueryMsg::GetItem {
                key: "meme".to_string(),
            },
        )
        .unwrap();
    let res: GetItemResponse = res.data().unwrap();

    assert_eq!(res.item, None);

    chain
        .orc
        .execute(
            "cwd_core",
            "exc_items_set",
            &cwd_core::msg::ExecuteMsg::ExecuteAdminMsgs {
                msgs: vec![CosmosMsg::Wasm(cosmwasm_std::WasmMsg::Execute {
                    contract_addr: dao.addr.clone(),
                    msg: to_binary(&cwd_core::msg::ExecuteMsg::SetItem {
                        key: "meme".to_string(),
                        addr: "foobar".to_string(),
                    })
                    .unwrap(),
                    funds: vec![],
                })],
            },
            &user_key,
            vec![],
        )
        .unwrap();

    let res = chain
        .orc
        .query(
            "cwd_core",
            &cwd_core::msg::QueryMsg::GetItem {
                key: "meme".to_string(),
            },
        )
        .unwrap();
    let res: GetItemResponse = res.data().unwrap();

    assert_eq!(res.item, Some("foobar".to_string()));

    // remove item:
    chain
        .orc
        .execute(
            "cwd_core",
            "exc_items_rm",
            &cwd_core::msg::ExecuteMsg::ExecuteAdminMsgs {
                msgs: vec![CosmosMsg::Wasm(cosmwasm_std::WasmMsg::Execute {
                    contract_addr: dao.addr,
                    msg: to_binary(&cwd_core::msg::ExecuteMsg::RemoveItem {
                        key: "meme".to_string(),
                    })
                    .unwrap(),
                    funds: vec![],
                })],
            },
            &user_key,
            vec![],
        )
        .unwrap();

    let res = chain
        .orc
        .query(
            "cwd_core",
            &cwd_core::msg::QueryMsg::GetItem {
                key: "meme".to_string(),
            },
        )
        .unwrap();
    let res: GetItemResponse = res.data().unwrap();

    assert_eq!(res.item, None);
}

// #### InstantiateMsg #####

#[test_context(Chain)]
#[test]
#[ignore]
fn instantiate_with_no_admin(chain: &mut Chain) {
    let user_addr = chain.users["user1"].account.address.clone();
    let user_key = chain.users["user1"].key.clone();

    let res = create_dao(chain, None, "inst_dao_no_admin", user_addr, &user_key);
    let dao = res.unwrap();

    // ensure the dao is the admin:
    assert_eq!(dao.state.admin, dao.addr);
    assert_eq!(dao.state.pause_info, PauseInfoResponse::Unpaused {});
    assert_eq!(
        dao.state.config,
        cwd_core::state::Config {
            dao_uri: None,
            name: "DAO DAO".to_string(),
            description: "A DAO that makes DAO tooling".to_string(),
            image_url: None,
            automatically_add_cw20s: false,
            automatically_add_cw721s: false
        }
    );
}

#[test_context(Chain)]
#[test]
#[ignore]
fn instantiate_with_admin(chain: &mut Chain) {
    let user_addr = chain.users["user1"].account.address.clone();
    let user_key = chain.users["user1"].key.clone();
    let voting_contract = "cwd_voting_cw20_staked";
    let proposal_contract = "cw_proposal_single";

    let res = create_dao(
        chain,
        Some(user_addr.clone()),
        "inst_admin_create_dao",
        user_addr.clone(),
        &user_key,
    );
    let dao = res.unwrap();

    // general dao info is valid:
    assert_eq!(dao.state.admin, user_addr);
    assert_eq!(dao.state.pause_info, PauseInfoResponse::Unpaused {});
    assert_eq!(
        dao.state.config,
        cwd_core::state::Config {
            dao_uri: None,
            name: "DAO DAO".to_string(),
            description: "A DAO that makes DAO tooling".to_string(),
            image_url: None,
            automatically_add_cw20s: false,
            automatically_add_cw721s: false
        }
    );

    let voting_addr = dao.state.voting_module.as_str();
    let prop_addr = dao.state.proposal_modules[0].address.as_str();

    // voting module config is valid:
    chain
        .orc
        .contract_map
        .add_address(voting_contract, voting_addr)
        .unwrap();
    let res = &chain
        .orc
        .query(
            voting_contract,
            &cwd_voting_cw20_staked::msg::QueryMsg::StakingContract {},
        )
        .unwrap();
    let staking_addr: &str = res.data().unwrap();

    chain
        .orc
        .contract_map
        .add_address("cw20_stake", staking_addr)
        .unwrap();
    let res = chain
        .orc
        .query(
            "cw20_stake",
            &cw20_stake::msg::QueryMsg::StakedValue { address: user_addr },
        )
        .unwrap();
    let staked_res: StakedValueResponse = res.data().unwrap();
    assert_eq!(staked_res.value, Uint128::new(0));

    let res = chain
        .orc
        .query("cw20_stake", &cw20_stake::msg::QueryMsg::GetConfig {})
        .unwrap();
    let config_res: cw20_stake::state::Config = res.data().unwrap();
    assert_eq!(
        config_res.owner,
        Some(Addr::unchecked(
            chain.orc.contract_map.address("cwd_core").unwrap()
        ))
    );
    assert_eq!(config_res.manager, None);

    let res = &chain
        .orc
        .query(
            voting_contract,
            &cwd_voting_cw20_staked::msg::QueryMsg::TokenContract {},
        )
        .unwrap();
    let token_addr: &str = res.data().unwrap();
    assert_eq!(config_res.token_address, token_addr);

    assert_eq!(config_res.unstaking_duration, Some(Duration::Time(1209600)));

    let res = chain
        .orc
        .query("cw20_stake", &cw20_stake::msg::QueryMsg::TotalValue {})
        .unwrap();
    let total_res: TotalValueResponse = res.data().unwrap();
    assert_eq!(total_res.total, Uint128::new(0));

    // proposal module config is valid:
    chain
        .orc
        .contract_map
        .add_address(proposal_contract, prop_addr)
        .unwrap();
    let res = chain
        .orc
        .query(
            proposal_contract,
            &cwd_proposal_single::msg::QueryMsg::Config {},
        )
        .unwrap();
    let config_res: cwd_proposal_single::state::Config = res.data().unwrap();
    let proposal_creation_policy: cwd_voting::pre_propose::ProposalCreationPolicy = chain
        .orc
        .query(
            proposal_contract,
            &cwd_proposal_single::msg::QueryMsg::ProposalCreationPolicy {},
        )
        .unwrap()
        .data()
        .unwrap();

    assert_eq!(config_res.min_voting_period, None);
    assert_eq!(config_res.max_voting_period, Duration::Time(432000));
    assert!(!config_res.allow_revoting);
    assert!(config_res.only_members_execute);
    assert!(matches!(
        proposal_creation_policy,
        ProposalCreationPolicy::Module { .. }
    ));
    assert_eq!(
        config_res.threshold,
        Threshold::ThresholdQuorum {
            threshold: PercentageThreshold::Majority {},
            quorum: PercentageThreshold::Percent(Decimal::percent(35)),
        }
    );
    assert_eq!(
        config_res.dao,
        chain.orc.contract_map.address("cwd_core").unwrap()
    );
}
