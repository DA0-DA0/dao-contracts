use core::time;
use std::thread;

use crate::helpers::{chain::Chain, helper::create_dao};
use cosm_orc::config::SigningKey;
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
    let staking_addr = &chain
        .orc
        .query(
            voting_contract,
            "exc_stake_q_stake",
            &cw20_staked_balance_voting::msg::QueryMsg::StakingContract {},
        )
        .unwrap()
        .data
        .unwrap();
    let staking_addr: String = serde_json::from_slice(staking_addr.value()).unwrap();

    chain
        .orc
        .contract_map
        .add_address("cw20_stake", staking_addr.to_string())
        .unwrap();
    let res = chain
        .orc
        .query(
            "cw20_stake",
            "exc_stake_q_cfg",
            &cw20_stake::msg::QueryMsg::StakedValue {
                address: account.to_string(),
            },
        )
        .unwrap();
    let staked_value: StakedValueResponse =
        serde_json::from_slice(res.data.as_ref().unwrap().value()).unwrap();

    assert_eq!(staked_value.value, Uint128::new(0));

    let res = chain
        .orc
        .query(
            "cw20_stake",
            "exc_stake_q_cfg",
            &cw20_stake::msg::QueryMsg::GetConfig {},
        )
        .unwrap();
    let config: Config = serde_json::from_slice(res.data.as_ref().unwrap().value()).unwrap();

    chain
        .orc
        .contract_map
        .add_address("cw20_base", config.token_address.as_str())
        .unwrap();
    chain
        .orc
        .execute(
            "cw20_base",
            "exc_stake_stake_tokens",
            &cw20_base::msg::ExecuteMsg::Send {
                contract: staking_addr,
                amount: Uint128::new(100),
                msg: to_binary(&cw20_stake::msg::ReceiveMsg::Stake {}).unwrap(),
            },
            &chain.key,
        )
        .unwrap();

    let res = chain
        .orc
        .query(
            "cw20_stake",
            "exc_stake_q_stake",
            &cw20_stake::msg::QueryMsg::StakedValue {
                address: account.to_string(),
            },
        )
        .unwrap();
    let staked_value: StakedValueResponse =
        serde_json::from_slice(res.data.unwrap().value()).unwrap();

    assert_eq!(staked_value.value, Uint128::new(100));

    // Sleep to let staking block process, so we have voting power:
    thread::sleep(time::Duration::from_millis(5000));

    let res = chain
        .orc
        .query(
            "cw_core",
            "exc_stake_q_power",
            &cw_core::msg::QueryMsg::VotingPowerAtHeight {
                address: account.to_string(),
                height: None,
            },
        )
        .unwrap();
    let power: VotingPowerAtHeightResponse =
        serde_json::from_slice(res.data.unwrap().value()).unwrap();

    assert_eq!(power.power, Uint128::new(100));
}
