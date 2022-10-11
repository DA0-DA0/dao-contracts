use crate::helpers::{chain::Chain, helper::create_dao};
use cosmwasm_std::{to_binary, Uint128};
use cw20_stake::{msg::StakedValueResponse, state::Config};
use cwd_interface::voting::VotingPowerAtHeightResponse;
use std::time::Duration;
use test_context::test_context;

// #### ExecuteMsg #####

#[test_context(Chain)]
#[test]
#[ignore]
fn execute_stake_tokens(chain: &mut Chain) {
    let user_addr = chain.users["user1"].account.address.clone();
    let user_key = chain.users["user1"].key.clone();
    let voting_contract = "cwd_voting_cw20_staked";

    let res = create_dao(
        chain,
        None,
        "exc_stake_create_dao",
        user_addr.clone(),
        &user_key,
    );
    let dao = res.unwrap();

    let voting_addr = dao.state.voting_module.as_str();

    // stake dao tokens:
    chain
        .orc
        .contract_map
        .add_address(voting_contract, voting_addr)
        .unwrap();
    let staking_addr: String = chain
        .orc
        .query(
            voting_contract,
            &cwd_voting_cw20_staked::msg::QueryMsg::StakingContract {},
        )
        .unwrap()
        .data()
        .unwrap();

    chain
        .orc
        .contract_map
        .add_address("cw20_stake", staking_addr.to_string())
        .unwrap();
    let res = chain
        .orc
        .query(
            "cw20_stake",
            &cw20_stake::msg::QueryMsg::StakedValue {
                address: user_addr.clone(),
            },
        )
        .unwrap();
    let staked_value: StakedValueResponse = res.data().unwrap();

    assert_eq!(staked_value.value, Uint128::new(0));

    let res = chain
        .orc
        .query("cw20_stake", &cw20_stake::msg::QueryMsg::GetConfig {})
        .unwrap();
    let config: Config = res.data().unwrap();

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
            &user_key,
            vec![],
        )
        .unwrap();

    let res = chain
        .orc
        .query(
            "cw20_stake",
            &cw20_stake::msg::QueryMsg::StakedValue {
                address: user_addr.clone(),
            },
        )
        .unwrap();
    let staked_value: StakedValueResponse = res.data().unwrap();

    assert_eq!(staked_value.value, Uint128::new(100));

    chain
        .orc
        .poll_for_n_blocks(1, Duration::from_millis(20_000), false)
        .unwrap();

    let res = chain
        .orc
        .query(
            "cwd_core",
            &cwd_core::msg::QueryMsg::VotingPowerAtHeight {
                address: user_addr,
                height: None,
            },
        )
        .unwrap();
    let power: VotingPowerAtHeightResponse = res.data().unwrap();

    assert_eq!(power.power, Uint128::new(100));
}
