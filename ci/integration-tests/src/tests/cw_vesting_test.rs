use std::{str::FromStr, time::Duration};

use cw_vesting::{
    msg::{ExecuteMsg, InstantiateMsg},
    vesting::Schedule,
};
use dao_chain_client::{Coin, Denom};

use cosmwasm_std::Uint128;
use test_context::test_context;

use crate::helpers::chain::Chain;

const CONTRACT_NAME: &str = "cw_vesting-staking";

// TODO CHECK INVALID ADDRESS (UN)DELEGATION ERRORS

/// Tests that tokens can be staked and rewards accumulated.
#[test_context(Chain)]
#[test]
#[ignore]
fn test_cw_vesting_staking(chain: &mut Chain) {
    let user_addr = chain.users["user3"].account.address.clone();
    let user_key = chain.users["user3"].key.clone();
    // key used for withdrawing delegator rewards so that we can
    // measure the rewards w/o txn cost included.
    let withdraw_key = chain.users["user5"].key.clone();

    let validator = chain
        .orc
        .bonded_validators()
        .unwrap()
        .into_iter()
        .next()
        .unwrap();
    eprintln!("delegating to: {validator}");

    chain
        .orc
        .instantiate(
            CONTRACT_NAME,
            "instantiate",
            &InstantiateMsg {
                owner: Some(user_addr.clone()),
                recipient: user_addr.to_string(),

                title: "title".to_string(),
                description: Some("description".to_string()),

                total: Uint128::new(100_000_000),
                denom: cw_vesting::UncheckedDenom::Native("ujuno".to_string()),

                schedule: Schedule::SaturatingLinear,
                start_time: None,
                vesting_duration_seconds: 10,
                unbonding_duration_seconds: 2 & 592000,
            },
            &user_key,
            None,
            vec![Coin {
                denom: Denom::from_str("ujuno").unwrap(),
                amount: 100_000_000,
            }],
        )
        .unwrap();

    // May not delegate to an invalid validator address.
    chain
        .orc
        .execute(
            CONTRACT_NAME,
            "delegate_and_error",
            &ExecuteMsg::Delegate {
                validator: "wowsorandom".to_string(),
                amount: Uint128::new(100_000_000),
            },
            &user_key,
            vec![],
        )
        .unwrap_err();

    chain
        .orc
        .execute(
            CONTRACT_NAME,
            "delegate",
            &ExecuteMsg::Delegate {
                validator: validator.clone(),
                amount: Uint128::new(100_000_000),
            },
            &user_key,
            vec![],
        )
        .unwrap();

    chain
        .orc
        .poll_for_n_blocks(3, Duration::from_secs(40), false)
        .unwrap();

    let start = chain.orc.balance(&user_addr, "ujuno").unwrap();

    chain
        .orc
        .execute(
            CONTRACT_NAME,
            "withdraw_reward",
            &ExecuteMsg::WithdrawDelegatorReward {
                validator: validator.clone(),
            },
            &withdraw_key,
            vec![],
        )
        .unwrap();

    let end = chain.orc.balance(&user_addr, "ujuno").unwrap();

    assert!(end > start, "{end} > {start}");

    // undelegate to complete the flow.
    chain
        .orc
        .execute(
            CONTRACT_NAME,
            "undelegate",
            &ExecuteMsg::Undelegate {
                validator,
                amount: Uint128::new(100_000_000),
            },
            &user_key,
            vec![],
        )
        .unwrap();
}
