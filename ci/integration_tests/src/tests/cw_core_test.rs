use crate::helpers::chain::Chain;
use crate::helpers::helper::{
    create_dao, CoreWasmMsg, Cw20StakeBalanceWasmMsg, Cw20StakeWasmMsg, CwProposalWasmMsg,
};
use cosm_orc::orchestrator::cosm_orc::WasmMsg;
use cosmwasm_std::{to_binary, Addr, CosmosMsg, Decimal, Uint128};
use cw20_stake::msg::{StakedValueResponse, TotalValueResponse};
use cw_core::query::{GetItemResponse, PauseInfoResponse};
use cw_utils::Duration;
use test_context::test_context;
use voting::{deposit::CheckedDepositInfo, threshold::PercentageThreshold, threshold::Threshold};

// #### ExecuteMsg #####

// TODO: Add tests for all cw-core execute msgs

#[test_context(Chain)]
#[test]
fn execute_execute_admin_msgs(chain: &mut Chain) {
    let user_addr = "juno10j9gpw9t4jsz47qgnkvl5n3zlm2fz72k67rxsg".to_string();

    // if you are not the admin, you cant execute admin msgs:
    let res = create_dao(chain, None, "exc_admin_msgs_create_dao", user_addr.clone());
    assert!(res.is_ok());
    let dao = res.unwrap();

    let msg: CoreWasmMsg = WasmMsg::ExecuteMsg(cw_core::msg::ExecuteMsg::ExecuteAdminMsgs {
        msgs: vec![CosmosMsg::Wasm(cosmwasm_std::WasmMsg::Execute {
            contract_addr: dao.addr,
            msg: to_binary(&cw_core::msg::ExecuteMsg::Pause {
                duration: Duration::Time(100),
            })
            .unwrap(),
            funds: vec![],
        })],
    });
    let res = chain
        .orc
        .process_msg("cw_core", "exc_admin_msgs_pause_dao_fail", &msg);
    assert!(res.is_err());

    let msg: CoreWasmMsg = WasmMsg::QueryMsg(cw_core::msg::QueryMsg::PauseInfo {});
    let res = chain
        .orc
        .process_msg("cw_core", "exc_admin_msgs_pause_dao_query", &msg)
        .unwrap();
    let res: PauseInfoResponse = serde_json::from_value(res["data"].clone()).unwrap();

    assert_eq!(res, PauseInfoResponse::Unpaused {});

    // if you are the admin you can execute admin msgs:
    let res = create_dao(
        chain,
        Some(user_addr.clone()),
        "exc_admin_msgs_create_dao_with_admin",
        user_addr,
    );
    assert!(res.is_ok());
    let dao = res.unwrap();

    let msgs: Vec<CoreWasmMsg> = vec![
        WasmMsg::ExecuteMsg(cw_core::msg::ExecuteMsg::ExecuteAdminMsgs {
            msgs: vec![CosmosMsg::Wasm(cosmwasm_std::WasmMsg::Execute {
                contract_addr: dao.addr,
                msg: to_binary(&cw_core::msg::ExecuteMsg::Pause {
                    duration: Duration::Height(100),
                })
                .unwrap(),
                funds: vec![],
            })],
        }),
        WasmMsg::QueryMsg(cw_core::msg::QueryMsg::PauseInfo {}),
    ];
    let res = chain
        .orc
        .process_msgs("cw_core", "exc_admin_msgs_pause_dao", &msgs)
        .unwrap();
    let res: PauseInfoResponse = serde_json::from_value(res[1]["data"].clone()).unwrap();
    assert_ne!(res, PauseInfoResponse::Unpaused {});
}

#[test_context(Chain)]
#[test]
fn execute_items(chain: &mut Chain) {
    let admin_addr = "juno10j9gpw9t4jsz47qgnkvl5n3zlm2fz72k67rxsg".to_string();

    // add item:
    let res = create_dao(
        chain,
        Some(admin_addr.clone()),
        "exc_items_create_dao",
        admin_addr,
    );
    assert!(res.is_ok());
    let dao = res.unwrap();

    let msg: CoreWasmMsg = WasmMsg::QueryMsg(cw_core::msg::QueryMsg::GetItem {
        key: "meme".to_string(),
    });
    let res = chain
        .orc
        .process_msg("cw_core", "exc_items_get", &msg)
        .unwrap();
    let res: GetItemResponse = serde_json::from_value(res["data"].clone()).unwrap();

    assert_eq!(res.item, None);

    let msgs: Vec<CoreWasmMsg> = vec![
        WasmMsg::ExecuteMsg(cw_core::msg::ExecuteMsg::ExecuteAdminMsgs {
            msgs: vec![CosmosMsg::Wasm(cosmwasm_std::WasmMsg::Execute {
                contract_addr: dao.addr.clone(),
                msg: to_binary(&cw_core::msg::ExecuteMsg::SetItem {
                    key: "meme".to_string(),
                    addr: "foobar".to_string(),
                })
                .unwrap(),
                funds: vec![],
            })],
        }),
        WasmMsg::QueryMsg(cw_core::msg::QueryMsg::GetItem {
            key: "meme".to_string(),
        }),
    ];

    let res = chain
        .orc
        .process_msgs("cw_core", "exc_items_set", &msgs)
        .unwrap();
    let res: GetItemResponse = serde_json::from_value(res[1]["data"].clone()).unwrap();

    assert_eq!(res.item, Some("foobar".to_string()));

    // remove item:
    let msgs: Vec<CoreWasmMsg> = vec![
        WasmMsg::ExecuteMsg(cw_core::msg::ExecuteMsg::ExecuteAdminMsgs {
            msgs: vec![CosmosMsg::Wasm(cosmwasm_std::WasmMsg::Execute {
                contract_addr: dao.addr,
                msg: to_binary(&cw_core::msg::ExecuteMsg::RemoveItem {
                    key: "meme".to_string(),
                })
                .unwrap(),
                funds: vec![],
            })],
        }),
        WasmMsg::QueryMsg(cw_core::msg::QueryMsg::GetItem {
            key: "meme".to_string(),
        }),
    ];

    let res = chain
        .orc
        .process_msgs("cw_core", "exc_items_rm", &msgs)
        .unwrap();
    let res: GetItemResponse = serde_json::from_value(res[1]["data"].clone()).unwrap();

    assert_eq!(res.item, None);
}

// #### InstantiateMsg #####

#[test_context(Chain)]
#[test]
fn instantiate_with_no_admin(chain: &mut Chain) {
    let user_addr = "juno10j9gpw9t4jsz47qgnkvl5n3zlm2fz72k67rxsg".to_string();

    let res = create_dao(chain, None, "inst_dao_no_admin", user_addr);
    assert!(res.is_ok());
    let dao = res.unwrap();

    // ensure the dao is the admin:
    assert_eq!(dao.state.admin, dao.addr);
    assert_eq!(dao.state.pause_info, PauseInfoResponse::Unpaused {});
    assert_eq!(
        dao.state.config,
        cw_core::state::Config {
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
fn instantiate_with_admin(chain: &mut Chain) {
    let admin_addr = "juno10j9gpw9t4jsz47qgnkvl5n3zlm2fz72k67rxsg".to_string();
    let voting_contract = "cw20_staked_balance_voting";
    let proposal_contract = "cw_proposal_single";

    let res = create_dao(
        chain,
        Some(admin_addr.clone()),
        "inst_admin_create_dao",
        admin_addr.clone(),
    );
    assert!(res.is_ok());
    let dao = res.unwrap();

    // general dao info is valid:
    assert_eq!(dao.state.admin, admin_addr);
    assert_eq!(dao.state.pause_info, PauseInfoResponse::Unpaused {});
    assert_eq!(
        dao.state.config,
        cw_core::state::Config {
            name: "DAO DAO".to_string(),
            description: "A DAO that makes DAO tooling".to_string(),
            image_url: None,
            automatically_add_cw20s: false,
            automatically_add_cw721s: false
        }
    );

    let voting_addr = dao.state.voting_module.as_str();
    let prop_addr = dao.state.proposal_modules[0].as_str();

    // voting module config is valid:
    chain
        .orc
        .contract_map
        .add_address(voting_contract, voting_addr)
        .unwrap();
    let msg: Cw20StakeBalanceWasmMsg =
        WasmMsg::QueryMsg(cw20_staked_balance_voting::msg::QueryMsg::StakingContract {});
    let staking_addr = &chain
        .orc
        .process_msg(voting_contract, "inst_admin_q_stake", &msg)
        .unwrap()["data"];

    chain
        .orc
        .contract_map
        .add_address("cw20_stake", staking_addr.as_str().unwrap())
        .unwrap();
    let msgs: Vec<Cw20StakeWasmMsg> = vec![
        WasmMsg::QueryMsg(cw20_stake::msg::QueryMsg::StakedValue {
            address: admin_addr,
        }),
        WasmMsg::QueryMsg(cw20_stake::msg::QueryMsg::GetConfig {}),
        WasmMsg::QueryMsg(cw20_stake::msg::QueryMsg::TotalValue {}),
    ];
    let res = chain
        .orc
        .process_msgs("cw20_stake", "inst_admin_q_val", &msgs)
        .unwrap();
    let staked_res: StakedValueResponse = serde_json::from_value(res[0]["data"].clone()).unwrap();
    assert_eq!(staked_res.value, Uint128::new(0));

    let config_res: cw20_stake::state::Config =
        serde_json::from_value(res[1]["data"].clone()).unwrap();
    assert_eq!(
        config_res.owner,
        Some(Addr::unchecked(
            chain.orc.contract_map.address("cw_core").unwrap()
        ))
    );
    assert_eq!(config_res.manager, None);

    let msg: Cw20StakeBalanceWasmMsg =
        WasmMsg::QueryMsg(cw20_staked_balance_voting::msg::QueryMsg::TokenContract {});
    let token_addr = &chain
        .orc
        .process_msg(voting_contract, "inst_admin_q_tok", &msg)
        .unwrap()["data"];
    let token_addr = token_addr.as_str().unwrap().to_string();
    assert_eq!(config_res.token_address, token_addr);

    assert_eq!(config_res.unstaking_duration, Some(Duration::Time(1209600)));

    let total_res: TotalValueResponse = serde_json::from_value(res[2]["data"].clone()).unwrap();
    assert_eq!(total_res.total, Uint128::new(0));

    // proposal module config is valid:
    chain
        .orc
        .contract_map
        .add_address(proposal_contract, prop_addr)
        .unwrap();
    let msg: CwProposalWasmMsg = WasmMsg::QueryMsg(cw_proposal_single::msg::QueryMsg::Config {});
    let res = chain
        .orc
        .process_msg(proposal_contract, "inst_admin_q_cfg", &msg)
        .unwrap();
    let config_res: cw_proposal_single::state::Config =
        serde_json::from_value(res["data"].clone()).unwrap();

    assert_eq!(config_res.min_voting_period, None);
    assert_eq!(config_res.max_voting_period, Duration::Time(432000));
    assert!(!config_res.allow_revoting);
    assert!(config_res.only_members_execute);
    assert_eq!(
        config_res.deposit_info,
        Some(CheckedDepositInfo {
            token: Addr::unchecked(token_addr),
            deposit: Uint128::new(1000000000),
            refund_failed_proposals: true,
        })
    );
    assert_eq!(
        config_res.threshold,
        Threshold::ThresholdQuorum {
            threshold: PercentageThreshold::Majority {},
            quorum: PercentageThreshold::Percent(Decimal::percent(35)),
        }
    );
    assert_eq!(
        config_res.dao,
        chain.orc.contract_map.address("cw_core").unwrap()
    );
}
