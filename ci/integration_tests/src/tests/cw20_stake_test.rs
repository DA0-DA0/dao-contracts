use core::time;
use std::thread;

use crate::helpers::{
    chain::Chain,
    helper::{create_dao, CoreWasmMsg, Cw20BaseWasmMsg, Cw20StakeBalanceWasmMsg, Cw20StakeWasmMsg},
};
use cosm_orc::{config::SigningKey, orchestrator::cosm_orc::WasmMsg};
use cosmwasm_std::{to_binary, Uint128};
use cw20_stake::{msg::StakedValueResponse, state::Config};
use cw_core_interface::voting::VotingPowerAtHeightResponse;
use test_context::test_context;

// #### ExecuteMsg #####

#[test_context(Chain)]
#[test]
fn execute_stake_tokens(chain: &mut Chain) {
    let key: SigningKey = chain.key.clone().try_into().unwrap();
    let account = key
        .public_key()
        .account_id(&chain.cfg.chain_cfg.prefix)
        .unwrap();
    let voting_contract = "cw20_staked_balance_voting";

    let res = create_dao(chain, None, "exc_stake_create_dao", account.to_string());
    assert!(res.is_ok());

    let dao = res.unwrap();

    let voting_addr = dao.state.voting_module.as_str();

    // stake dao tokens:
    chain
        .orc
        .contract_map
        .add_address(voting_contract, voting_addr)
        .unwrap();
    let msg: Cw20StakeBalanceWasmMsg =
        WasmMsg::QueryMsg(cw20_staked_balance_voting::msg::QueryMsg::StakingContract {});
    let staking_addr = &chain
        .orc
        .process_msg(voting_contract, "exc_stake_q_stake", &msg, &chain.key)
        .unwrap()
        .data
        .unwrap();
    let staking_addr: String = serde_json::from_slice(staking_addr.value()).unwrap();

    chain
        .orc
        .contract_map
        .add_address("cw20_stake", staking_addr.to_string())
        .unwrap();
    let msgs: Vec<Cw20StakeWasmMsg> = vec![
        WasmMsg::QueryMsg(cw20_stake::msg::QueryMsg::StakedValue {
            address: account.to_string(),
        }),
        WasmMsg::QueryMsg(cw20_stake::msg::QueryMsg::GetConfig {}),
    ];
    let res = chain
        .orc
        .process_msgs("cw20_stake", "exc_stake_q_cfg", &msgs, &chain.key)
        .unwrap();
    let staked_value: StakedValueResponse =
        serde_json::from_slice(res[0].data.as_ref().unwrap().value()).unwrap();

    assert_eq!(staked_value.value, Uint128::new(0));

    let config: Config = serde_json::from_slice(res[1].data.as_ref().unwrap().value()).unwrap();
    chain
        .orc
        .contract_map
        .add_address("cw20_base", config.token_address.as_str())
        .unwrap();
    let msg: Cw20BaseWasmMsg = WasmMsg::ExecuteMsg(cw20_base::msg::ExecuteMsg::Send {
        contract: staking_addr,
        amount: Uint128::new(100),
        msg: to_binary(&cw20_stake::msg::ReceiveMsg::Stake {}).unwrap(),
    });
    chain
        .orc
        .process_msg("cw20_base", "exc_stake_stake_tokens", &msg, &chain.key)
        .unwrap();

    let msg: Cw20StakeWasmMsg = WasmMsg::QueryMsg(cw20_stake::msg::QueryMsg::StakedValue {
        address: account.to_string(),
    });
    let res = chain
        .orc
        .process_msg("cw20_stake", "exc_stake_q_stake", &msg, &chain.key)
        .unwrap();
    let staked_value: StakedValueResponse =
        serde_json::from_slice(res.data.unwrap().value()).unwrap();

    assert_eq!(staked_value.value, Uint128::new(100));

    // Sleep to let staking block process, so we have voting power:
    thread::sleep(time::Duration::from_millis(5000));

    let msg: CoreWasmMsg = WasmMsg::QueryMsg(cw_core::msg::QueryMsg::VotingPowerAtHeight {
        address: account.to_string(),
        height: None,
    });
    let res = chain
        .orc
        .process_msg("cw_core", "exc_stake_q_power", &msg, &chain.key)
        .unwrap();
    let power: VotingPowerAtHeightResponse =
        serde_json::from_slice(res.data.unwrap().value()).unwrap();

    assert_eq!(power.power, Uint128::new(100));
}
