use core::time;
use std::thread;

use crate::{
    helpers::helper::{
        create_dao, CoreWasmMsg, Cw20BaseWasmMsg, Cw20StakeBalanceWasmMsg, Cw20StakeWasmMsg,
    },
    test_harness::chain::Chain,
};
use cosm_orc::orchestrator::cosm_orc::WasmMsg;
use cosmwasm_std::{to_binary, Uint128};
use cw20_stake::{msg::StakedValueResponse, state::Config};
use cw_core_interface::voting::VotingPowerAtHeightResponse;

// #### ExecuteMsg #####

#[test]
fn execute_stake_tokens() {
    let user_addr = "juno10j9gpw9t4jsz47qgnkvl5n3zlm2fz72k67rxsg".to_string();
    let voting_contract = "cw20_staked_balance_voting";
    let proposal_contract = "cw_proposal_single";

    let dao = create_dao(None, user_addr.clone(), voting_contract, proposal_contract);

    let voting_addr = dao.state.voting_module.as_str();

    // stake dao tokens:
    Chain::add_deploy_code_addr(voting_contract, voting_addr);
    let msg: Cw20StakeBalanceWasmMsg =
        WasmMsg::QueryMsg(cw20_staked_balance_voting::msg::QueryMsg::StakingContract {});
    let staking_addr = &Chain::process_msg(voting_contract.to_string(), &msg).unwrap()["data"];
    let staking_addr = staking_addr.as_str().unwrap();

    Chain::add_deploy_code_addr("cw20_stake", staking_addr);
    let msgs: Vec<Cw20StakeWasmMsg> = vec![
        WasmMsg::QueryMsg(cw20_stake::msg::QueryMsg::StakedValue {
            address: user_addr.clone(),
        }),
        WasmMsg::QueryMsg(cw20_stake::msg::QueryMsg::GetConfig {}),
    ];
    let res = Chain::process_msgs("cw20_stake".to_string(), &msgs).unwrap();
    let staked_value: StakedValueResponse = serde_json::from_value(res[0]["data"].clone()).unwrap();

    assert_eq!(staked_value.value, Uint128::new(0));

    let config: Config = serde_json::from_value(res[1]["data"].clone()).unwrap();
    Chain::add_deploy_code_addr("cw20_base", config.token_address.as_str());
    let msg: Cw20BaseWasmMsg = WasmMsg::ExecuteMsg(cw20_base::msg::ExecuteMsg::Send {
        contract: staking_addr.to_string(),
        amount: Uint128::new(100),
        msg: to_binary(&cw20_stake::msg::ReceiveMsg::Stake {}).unwrap(),
    });
    Chain::process_msg("cw20_base".to_string(), &msg).unwrap();

    let msg: Cw20StakeWasmMsg = WasmMsg::QueryMsg(cw20_stake::msg::QueryMsg::StakedValue {
        address: user_addr.clone(),
    });
    let res = Chain::process_msg("cw20_stake".to_string(), &msg).unwrap();
    let staked_value: StakedValueResponse = serde_json::from_value(res["data"].clone()).unwrap();

    assert_eq!(staked_value.value, Uint128::new(100));

    // Sleep to let staking block process, so we have voting power:
    thread::sleep(time::Duration::from_millis(5000));

    let msg: CoreWasmMsg = WasmMsg::QueryMsg(cw_core::msg::QueryMsg::VotingPowerAtHeight {
        address: user_addr.clone(),
        height: None,
    });
    let res = Chain::process_msg("cw_core".to_string(), &msg).unwrap();
    let power: VotingPowerAtHeightResponse = serde_json::from_value(res["data"].clone()).unwrap();

    assert_eq!(power.power, Uint128::new(100));
}
