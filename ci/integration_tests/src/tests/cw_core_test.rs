use crate::{
    helpers::helpers::{
        create_dao, CoreWasmMsg, Cw20StakeBalanceWasmMsg, Cw20StakeWasmMsg, CwProposalWasmMsg,
    },
    test_harness::chain::Chain,
};
use cosm_orc::orchestrator::cosm_orc::WasmMsg;
use cosmwasm_std::{to_binary, Addr, CosmosMsg, Decimal, Uint128};
use cw20_stake::msg::{StakedValueResponse, TotalValueResponse};
use cw_core::query::{GetItemResponse, PauseInfoResponse};
use cw_utils::Duration;
use voting::{deposit::CheckedDepositInfo, threshold::PercentageThreshold, threshold::Threshold};

// #### ExecuteMsg #####

// TODO: Add tests for all cw-core execute msgs

#[test]
fn execute_execute_admin_msgs() {
    let user_addr = "juno10j9gpw9t4jsz47qgnkvl5n3zlm2fz72k67rxsg".to_string();

    // dao without an admin cannot execute admin msgs:
    let dao = create_dao(
        None,
        user_addr.clone(),
        None,
        "cw20_staked_balance_voting",
        "cw_proposal_single",
    );

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
    let res = Chain::process_msg("cw_core".to_string(), &msg);
    assert!(res.is_err());

    // dao with admin can execute admin msgs:
    let dao = create_dao(
        Some(user_addr.clone()),
        user_addr,
        None,
        "cw20_staked_balance_voting",
        "cw_proposal_single",
    );

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
    let res = Chain::process_msgs("cw_core".to_string(), &msgs).unwrap();
    let res: PauseInfoResponse = serde_json::from_value(res[1]["data"].clone()).unwrap();
    assert_ne!(res, PauseInfoResponse::Unpaused {});
}

#[test]
fn execute_items() {
    let admin_addr = "juno10j9gpw9t4jsz47qgnkvl5n3zlm2fz72k67rxsg".to_string();

    let dao = create_dao(
        Some(admin_addr.clone()),
        admin_addr,
        None,
        "cw20_staked_balance_voting",
        "cw_proposal_single",
    );

    let msg: CoreWasmMsg = WasmMsg::QueryMsg(cw_core::msg::QueryMsg::GetItem {
        key: "meme".to_string(),
    });
    let res = Chain::process_msg("cw_core".to_string(), &msg).unwrap();
    let res: GetItemResponse = serde_json::from_value(res["data"].clone()).unwrap();

    assert_eq!(res.item, None);

    let msgs: Vec<CoreWasmMsg> = vec![
        WasmMsg::ExecuteMsg(cw_core::msg::ExecuteMsg::ExecuteAdminMsgs {
            msgs: vec![CosmosMsg::Wasm(cosmwasm_std::WasmMsg::Execute {
                contract_addr: dao.addr,
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

    let res = Chain::process_msgs("cw_core".to_string(), &msgs).unwrap();
    let res: GetItemResponse = serde_json::from_value(res[1]["data"].clone()).unwrap();

    assert_eq!(res.item, Some("foobar".to_string()));

    // TODO: remove item
}

// #### InstantiateMsg #####

#[test]
fn instantiate_with_no_admin() {
    let user_addr = "juno10j9gpw9t4jsz47qgnkvl5n3zlm2fz72k67rxsg".to_string();

    let dao = create_dao(
        None,
        user_addr.clone(),
        None,
        "cw20_staked_balance_voting",
        "cw_proposal_single",
    );

    // ensure the dao is the admin:
    assert_eq!(dao.state.admin, Chain::deploy_code_addr("cw_core"));
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

#[test]
fn instantiate_with_admin() {
    let admin_addr = "juno10j9gpw9t4jsz47qgnkvl5n3zlm2fz72k67rxsg".to_string();
    let voting_contract = "cw20_staked_balance_voting";
    let proposal_contract = "cw_proposal_single";

    let dao = create_dao(
        Some(admin_addr.clone()),
        admin_addr.clone(),
        None,
        voting_contract,
        proposal_contract,
    );

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
    Chain::add_deploy_code_addr(voting_contract, voting_addr);
    let msg: Cw20StakeBalanceWasmMsg =
        WasmMsg::QueryMsg(cw20_staked_balance_voting::msg::QueryMsg::StakingContract {});
    let staking_addr = &Chain::process_msg(voting_contract.to_string(), &msg).unwrap()["data"];

    Chain::add_deploy_code_addr("cw20_stake", staking_addr.as_str().unwrap());
    let msgs: Vec<Cw20StakeWasmMsg> = vec![
        WasmMsg::QueryMsg(cw20_stake::msg::QueryMsg::StakedValue {
            address: admin_addr.to_string(),
        }),
        WasmMsg::QueryMsg(cw20_stake::msg::QueryMsg::GetConfig {}),
        WasmMsg::QueryMsg(cw20_stake::msg::QueryMsg::TotalValue {}),
    ];
    let res = Chain::process_msgs("cw20_stake".to_string(), &msgs).unwrap();
    let staked_res: StakedValueResponse = serde_json::from_value(res[0]["data"].clone()).unwrap();
    assert_eq!(staked_res.value, Uint128::new(0));

    let config_res: cw20_stake::state::Config =
        serde_json::from_value(res[1]["data"].clone()).unwrap();
    assert_eq!(
        config_res.owner,
        Some(Addr::unchecked(Chain::deploy_code_addr("cw_core")))
    );
    assert_eq!(config_res.manager, None);

    let msg: Cw20StakeBalanceWasmMsg =
        WasmMsg::QueryMsg(cw20_staked_balance_voting::msg::QueryMsg::TokenContract {});
    let token_addr = &Chain::process_msg(voting_contract.to_string(), &msg).unwrap()["data"];
    let token_addr = token_addr.as_str().unwrap().to_string();
    assert_eq!(config_res.token_address, token_addr);

    assert_eq!(config_res.unstaking_duration, Some(Duration::Time(1209600)));

    let total_res: TotalValueResponse = serde_json::from_value(res[2]["data"].clone()).unwrap();
    assert_eq!(total_res.total, Uint128::new(0));

    // proposal module config is valid:
    Chain::add_deploy_code_addr(proposal_contract, prop_addr);
    let msg: CwProposalWasmMsg = WasmMsg::QueryMsg(cw_proposal_single::msg::QueryMsg::Config {});
    let res = Chain::process_msg(proposal_contract.to_string(), &msg).unwrap();
    let config_res: cw_proposal_single::state::Config =
        serde_json::from_value(res["data"].clone()).unwrap();

    assert_eq!(config_res.min_voting_period, None);
    assert_eq!(config_res.max_voting_period, Duration::Time(432000));
    assert_eq!(config_res.allow_revoting, false);
    assert_eq!(config_res.only_members_execute, true);
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
    assert_eq!(config_res.dao, Chain::deploy_code_addr("cw_core"));
}
