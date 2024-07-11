use std::borrow::BorrowMut;

use cosmwasm_std::Uint128;
use cosmwasm_std::{coin, to_json_binary, Addr, Timestamp};
use cw20::{Cw20Coin, Expiration, UncheckedDenom};
use cw4::Member;
use cw_multi_test::Executor;
use cw_utils::Duration;

use crate::{
    msg::ExecuteMsg,
    testing::{ADDR1, ADDR2, ADDR3, DENOM},
};

use super::{
    suite::{RewardsConfig, SuiteBuilder},
    ALT_DENOM, OWNER,
};

// By default, the tests are set up to distribute rewards over 1_000_000 units of time.
// Over that time, 100_000_000 token rewards will be distributed.

#[test]
fn test_cw20_dao_native_rewards_block_height_based() {
    let mut suite = SuiteBuilder::base(super::suite::DaoType::CW20).build();

    suite.assert_amount(1_000);
    suite.assert_ends_at(Expiration::AtHeight(1_000_000));
    suite.assert_duration(10);

    // skip 1/10th of the time
    suite.skip_blocks(100_000);

    suite.assert_pending_rewards(ADDR1, DENOM, 5_000_000);
    suite.assert_pending_rewards(ADDR2, DENOM, 2_500_000);
    suite.assert_pending_rewards(ADDR3, DENOM, 2_500_000);

    // skip 1/10th of the time
    suite.skip_blocks(100_000);

    suite.assert_pending_rewards(ADDR1, DENOM, 10_000_000);
    suite.assert_pending_rewards(ADDR2, DENOM, 5_000_000);
    suite.assert_pending_rewards(ADDR3, DENOM, 5_000_000);

    // ADDR1 claims rewards
    suite.claim_rewards(ADDR1, DENOM);
    suite.assert_native_balance(ADDR1, DENOM, 10_000_000);
    suite.assert_pending_rewards(ADDR1, DENOM, 0);

    // ADDR2 and ADDR3 unstake their rewards
    suite.unstake_cw20_tokens(50, ADDR2);
    suite.unstake_cw20_tokens(50, ADDR3);

    // skip 1/10th of the time
    suite.skip_blocks(100_000);

    // because ADDR2 and ADDR3 are not staking, ADDR1 receives all the rewards.
    // ADDR2 and ADDR3 should have the same amount of pending rewards as before.
    suite.assert_pending_rewards(ADDR1, DENOM, 10_000_000);
    suite.assert_pending_rewards(ADDR2, DENOM, 5_000_000);
    suite.assert_pending_rewards(ADDR3, DENOM, 5_000_000);

    // ADDR2 and ADDR3 wake up, claim and restake their rewards
    suite.claim_rewards(ADDR2, DENOM);
    suite.claim_rewards(ADDR3, DENOM);

    suite.stake_cw20_tokens(50, ADDR2);

    // skip 3/10th of the time
    suite.skip_blocks(300_000);

    suite.stake_cw20_tokens(50, ADDR3);

    suite.assert_pending_rewards(ADDR1, DENOM, 30_000_000);
    suite.assert_pending_rewards(ADDR2, DENOM, 10_000_000);
    suite.assert_pending_rewards(ADDR3, DENOM, 0);

    suite.claim_rewards(ADDR1, DENOM);
    suite.claim_rewards(ADDR2, DENOM);

    suite.assert_pending_rewards(ADDR1, DENOM, 0);
    suite.assert_pending_rewards(ADDR2, DENOM, 0);
    suite.assert_pending_rewards(ADDR3, DENOM, 0);

    let remaining_time = suite.get_time_until_rewards_expiration();

    suite.skip_blocks(remaining_time - 100_000);

    suite.claim_rewards(ADDR1, DENOM);
    suite.unstake_cw20_tokens(100, ADDR1);
    suite.assert_pending_rewards(ADDR1, DENOM, 0);

    suite.skip_blocks(100_000);

    suite.unstake_cw20_tokens(50, ADDR2);
    suite.skip_blocks(100_000);

    suite.claim_rewards(ADDR2, DENOM);
    suite.claim_rewards(ADDR3, DENOM);

    suite.assert_pending_rewards(ADDR1, DENOM, 0);
    suite.assert_pending_rewards(ADDR2, DENOM, 0);
    suite.assert_pending_rewards(ADDR3, DENOM, 0);

    let addr1_bal = suite.get_balance_native(ADDR1, DENOM);
    let addr2_bal = suite.get_balance_native(ADDR2, DENOM);
    let addr3_bal = suite.get_balance_native(ADDR3, DENOM);

    println!("Balances: {}, {}, {}", addr1_bal, addr2_bal, addr3_bal);
}

#[test]
fn test_cw721_dao_rewards() {
    let mut suite = SuiteBuilder::base(super::suite::DaoType::CW721).build();

    suite.assert_amount(1_000);
    suite.assert_ends_at(Expiration::AtHeight(1_000_000));
    suite.assert_duration(10);

    // skip 1/10th of the time
    suite.skip_blocks(100_000);

    suite.assert_pending_rewards(ADDR1, DENOM, 5_000_000);
    suite.assert_pending_rewards(ADDR2, DENOM, 2_500_000);
    suite.assert_pending_rewards(ADDR3, DENOM, 2_500_000);

    // skip 1/10th of the time
    suite.skip_blocks(100_000);

    suite.assert_pending_rewards(ADDR1, DENOM, 10_000_000);
    suite.assert_pending_rewards(ADDR2, DENOM, 5_000_000);
    suite.assert_pending_rewards(ADDR3, DENOM, 5_000_000);

    // ADDR1 claims rewards
    suite.claim_rewards(ADDR1, DENOM);
    suite.assert_native_balance(ADDR1, DENOM, 10_000_000);
    suite.assert_pending_rewards(ADDR1, DENOM, 0);

    // ADDR2 and ADDR3 unstake their nfts
    suite.unstake_nft(ADDR2, 3);
    suite.unstake_nft(ADDR3, 4);

    // skip 1/10th of the time
    suite.skip_blocks(100_000);

    // because ADDR2 and ADDR3 are not staking, ADDR1 receives all the rewards.
    // ADDR2 and ADDR3 should have the same amount of pending rewards as before.
    suite.assert_pending_rewards(ADDR1, DENOM, 10_000_000);
    suite.assert_pending_rewards(ADDR2, DENOM, 5_000_000);
    suite.assert_pending_rewards(ADDR3, DENOM, 5_000_000);

    // ADDR2 and ADDR3 wake up, claim and restake their nfts
    suite.claim_rewards(ADDR2, DENOM);
    suite.claim_rewards(ADDR3, DENOM);

    suite.stake_nft(ADDR2, 3);
    suite.stake_nft(ADDR3, 4);
}

#[test]
#[should_panic(expected = "No rewards claimable")]
fn test_claim_zero_rewards() {
    let mut suite = SuiteBuilder::base(super::suite::DaoType::CW20).build();

    // skip 1/10th of the time
    suite.skip_blocks(100_000);

    suite.assert_pending_rewards(ADDR1, DENOM, 5_000_000);

    // ADDR1 claims rewards
    suite.claim_rewards(ADDR1, DENOM);

    // ADDR1 attempts to claim again
    suite.claim_rewards(ADDR1, DENOM);
}

#[test]
fn test_native_dao_cw20_rewards_time_based() {
    // 1000udenom/10sec = 100udenom/1sec reward emission rate
    // given funding of 100_000_000udenom, we have a reward duration of 1_000_000sec
    let mut suite = SuiteBuilder::base(super::suite::DaoType::CW20)
        .with_rewards_config(RewardsConfig {
            amount: 1_000,
            denom: UncheckedDenom::Cw20(DENOM.to_string()),
            duration: Duration::Time(10),
            destination: None,
        })
        .build();

    suite.assert_amount(1_000);
    suite.assert_duration(10);
    suite.assert_ends_at(Expiration::AtTime(Timestamp::from_seconds(1_000_000)));

    // skip 1/10th of the time
    suite.skip_seconds(100_000);

    // suite.assert_pending_rewards(ADDR1, DENOM, 5_000_000);
    suite.assert_pending_rewards(ADDR2, DENOM, 2_500_000);
    suite.assert_pending_rewards(ADDR3, DENOM, 2_500_000);

    // skip 1/10th of the time
    suite.skip_seconds(100_000);

    suite.assert_pending_rewards(ADDR1, DENOM, 10_000_000);
    suite.assert_pending_rewards(ADDR2, DENOM, 5_000_000);
    suite.assert_pending_rewards(ADDR3, DENOM, 5_000_000);

    // ADDR1 claims rewards
    suite.claim_rewards(ADDR1, suite.reward_denom.clone().as_str());
    suite.assert_cw20_balance(ADDR1, 10_000_000);
    suite.assert_pending_rewards(ADDR1, DENOM, 0);

    // ADDR2 and ADDR3 unstake their stake
    suite.unstake_cw20_tokens(50, ADDR2);
    suite.unstake_cw20_tokens(50, ADDR3);

    // skip 1/10th of the time
    suite.skip_seconds(100_000);

    // because ADDR2 and ADDR3 are not staking, ADDR1 receives all the rewards.
    // ADDR2 and ADDR3 should have the same amount of pending rewards as before.
    suite.assert_pending_rewards(ADDR1, DENOM, 10_000_000);
    suite.assert_pending_rewards(ADDR2, DENOM, 5_000_000);
    suite.assert_pending_rewards(ADDR3, DENOM, 5_000_000);

    // ADDR2 and ADDR3 wake up and claim their rewards
    suite.claim_rewards(ADDR2, suite.reward_denom.clone().as_str());
    suite.claim_rewards(ADDR3, suite.reward_denom.clone().as_str());

    suite.assert_cw20_balance(ADDR1, 10_000_000);
    suite.assert_cw20_balance(ADDR2, 5_000_000);
}

#[test]
fn test_native_dao_rewards_time_based() {
    // 1000udenom/10sec = 100udenom/1sec reward emission rate
    // given funding of 100_000_000udenom, we have a reward duration of 1_000_000sec
    let mut suite = SuiteBuilder::base(super::suite::DaoType::Native)
        .with_rewards_config(RewardsConfig {
            amount: 1_000,
            denom: UncheckedDenom::Native(DENOM.to_string()),
            duration: Duration::Time(10),
            destination: None,
        })
        .build();

    suite.assert_amount(1_000);
    suite.assert_duration(10);
    suite.assert_ends_at(Expiration::AtTime(Timestamp::from_seconds(1_000_000)));

    // skip 1/10th of the time
    suite.skip_seconds(100_000);

    // suite.assert_pending_rewards(ADDR1, DENOM, 5_000_000);
    suite.assert_pending_rewards(ADDR2, DENOM, 2_500_000);
    suite.assert_pending_rewards(ADDR3, DENOM, 2_500_000);

    // skip 1/10th of the time
    suite.skip_seconds(100_000);

    suite.assert_pending_rewards(ADDR1, DENOM, 10_000_000);
    suite.assert_pending_rewards(ADDR2, DENOM, 5_000_000);
    suite.assert_pending_rewards(ADDR3, DENOM, 5_000_000);

    // ADDR1 claims rewards
    suite.claim_rewards(ADDR1, DENOM);
    suite.assert_native_balance(ADDR1, DENOM, 10_000_000);
    suite.assert_pending_rewards(ADDR1, DENOM, 0);

    // ADDR2 and ADDR3 unstake their stake
    suite.unstake_native_tokens(ADDR2, 50);
    suite.unstake_native_tokens(ADDR3, 50);

    // skip 1/10th of the time
    suite.skip_seconds(100_000);

    // because ADDR2 and ADDR3 are not staking, ADDR1 receives all the rewards.
    // ADDR2 and ADDR3 should have the same amount of pending rewards as before.
    suite.assert_pending_rewards(ADDR1, DENOM, 10_000_000);
    suite.assert_pending_rewards(ADDR2, DENOM, 5_000_000);
    suite.assert_pending_rewards(ADDR3, DENOM, 5_000_000);

    // ADDR2 and ADDR3 wake up, claim and restake their rewards
    suite.claim_rewards(ADDR2, DENOM);
    suite.claim_rewards(ADDR3, DENOM);

    let addr1_balance = suite.get_balance_native(ADDR1, DENOM);
    let addr2_balance = suite.get_balance_native(ADDR2, DENOM);

    suite.stake_native_tokens(ADDR1, addr1_balance);
    suite.stake_native_tokens(ADDR2, addr2_balance);
}

#[test]
fn test_native_dao_rewards() {
    let mut suite = SuiteBuilder::base(super::suite::DaoType::Native).build();

    suite.assert_amount(1_000);
    suite.assert_ends_at(Expiration::AtHeight(1_000_000));
    suite.assert_duration(10);

    // skip 1/10th of the time
    suite.skip_blocks(100_000);

    suite.assert_pending_rewards(ADDR1, DENOM, 5_000_000);
    suite.assert_pending_rewards(ADDR2, DENOM, 2_500_000);
    suite.assert_pending_rewards(ADDR3, DENOM, 2_500_000);

    // skip 1/10th of the time
    suite.skip_blocks(100_000);

    suite.assert_pending_rewards(ADDR1, DENOM, 10_000_000);
    suite.assert_pending_rewards(ADDR2, DENOM, 5_000_000);
    suite.assert_pending_rewards(ADDR3, DENOM, 5_000_000);

    // ADDR1 claims rewards
    suite.claim_rewards(ADDR1, DENOM);
    suite.assert_native_balance(ADDR1, DENOM, 10_000_000);
    suite.assert_pending_rewards(ADDR1, DENOM, 0);

    // ADDR2 and ADDR3 unstake their stake
    suite.unstake_native_tokens(ADDR2, 50);
    suite.unstake_native_tokens(ADDR3, 50);

    // skip 1/10th of the time
    suite.skip_blocks(100_000);

    // because ADDR2 and ADDR3 are not staking, ADDR1 receives all the rewards.
    // ADDR2 and ADDR3 should have the same amount of pending rewards as before.
    suite.assert_pending_rewards(ADDR1, DENOM, 10_000_000);
    suite.assert_pending_rewards(ADDR2, DENOM, 5_000_000);
    suite.assert_pending_rewards(ADDR3, DENOM, 5_000_000);

    // ADDR2 and ADDR3 wake up, claim and restake their rewards
    suite.claim_rewards(ADDR2, DENOM);
    suite.claim_rewards(ADDR3, DENOM);

    let addr1_balance = suite.get_balance_native(ADDR1, DENOM);
    let addr2_balance = suite.get_balance_native(ADDR2, DENOM);

    suite.stake_native_tokens(ADDR1, addr1_balance);
    suite.stake_native_tokens(ADDR2, addr2_balance);
}

#[test]
fn test_cw4_dao_rewards() {
    let mut suite = SuiteBuilder::base(super::suite::DaoType::CW4).build();

    suite.assert_amount(1_000);
    suite.assert_ends_at(Expiration::AtHeight(1_000_000));
    suite.assert_duration(10);

    // skip 1/10th of the time
    suite.skip_blocks(100_000);

    suite.assert_pending_rewards(ADDR1, DENOM, 5_000_000);
    suite.assert_pending_rewards(ADDR2, DENOM, 2_500_000);
    suite.assert_pending_rewards(ADDR3, DENOM, 2_500_000);

    // remove the second member
    suite.update_members(vec![], vec![ADDR2.to_string()]);
    suite.query_members();

    // skip 1/10th of the time
    suite.skip_blocks(100_000);

    // now that ADDR2 is no longer a member, ADDR1 and ADDR3 will split the rewards
    suite.assert_pending_rewards(ADDR1, DENOM, 11_666_666);
    suite.assert_pending_rewards(ADDR2, DENOM, 2_500_000);
    suite.assert_pending_rewards(ADDR3, DENOM, 5_833_333);

    // reintroduce the 2nd member with double the vp
    let add_member_2 = Member {
        addr: ADDR2.to_string(),
        weight: 2,
    };
    suite.update_members(vec![add_member_2], vec![]);
    suite.query_members();

    // now the vp split is [ADDR1: 40%, ADDR2: 40%, ADDR3: 20%]
    // meaning the token reward per 100k blocks is 4mil, 4mil, 2mil

    // ADDR1 claims rewards
    suite.claim_rewards(ADDR1, DENOM);
    suite.assert_native_balance(ADDR1, DENOM, 11_666_666);

    // assert pending rewards are still the same (other than ADDR1)
    suite.assert_pending_rewards(ADDR1, DENOM, 0);
    suite.assert_pending_rewards(ADDR2, DENOM, 2_500_000);
    suite.assert_pending_rewards(ADDR3, DENOM, 5_833_333);

    // skip 1/10th of the time
    suite.skip_blocks(100_000);

    suite.assert_pending_rewards(ADDR1, DENOM, 4_000_000);
    suite.assert_pending_rewards(ADDR2, DENOM, 6_500_000);
    suite.assert_pending_rewards(ADDR3, DENOM, 7_833_333);

    // skip 1/2 of time, leaving 200k blocks left
    suite.skip_blocks(500_000);

    suite.assert_pending_rewards(ADDR1, DENOM, 24_000_000);
    suite.assert_pending_rewards(ADDR2, DENOM, 26_500_000);
    suite.assert_pending_rewards(ADDR3, DENOM, 17_833_333);

    // remove all members
    suite.update_members(
        vec![],
        vec![ADDR1.to_string(), ADDR2.to_string(), ADDR3.to_string()],
    );

    suite.assert_pending_rewards(ADDR1, DENOM, 24_000_000);
    suite.assert_pending_rewards(ADDR2, DENOM, 26_500_000);
    suite.assert_pending_rewards(ADDR3, DENOM, 17_833_333);

    // skip 1/10th of the time
    suite.skip_blocks(100_000);

    suite.assert_pending_rewards(ADDR1, DENOM, 24_000_000);
    suite.assert_pending_rewards(ADDR2, DENOM, 26_500_000);
    suite.assert_pending_rewards(ADDR3, DENOM, 17_833_333);

    suite.update_members(
        vec![
            Member {
                addr: ADDR1.to_string(),
                weight: 2,
            },
            Member {
                addr: ADDR2.to_string(),
                weight: 2,
            },
            Member {
                addr: ADDR3.to_string(),
                weight: 1,
            },
        ],
        vec![],
    );

    suite.assert_pending_rewards(ADDR1, DENOM, 24_000_000);
    suite.assert_pending_rewards(ADDR2, DENOM, 26_500_000);
    suite.assert_pending_rewards(ADDR3, DENOM, 17_833_333);

    suite.claim_rewards(ADDR1, DENOM);
    suite.assert_pending_rewards(ADDR1, DENOM, 0);
    suite.assert_native_balance(ADDR1, DENOM, 35_666_666);

    // skip 1/10th of the time
    suite.skip_blocks(100_000);

    suite.assert_pending_rewards(ADDR1, DENOM, 4_000_000);
    suite.assert_pending_rewards(ADDR2, DENOM, 30_500_000);
    suite.assert_pending_rewards(ADDR3, DENOM, 19_833_333);

    // at the very expiration block, claim rewards
    suite.claim_rewards(ADDR2, DENOM);
    suite.assert_pending_rewards(ADDR2, DENOM, 0);
    suite.assert_native_balance(ADDR2, DENOM, 30_500_000);

    suite.skip_blocks(100_000);

    suite.claim_rewards(ADDR1, DENOM);
    suite.claim_rewards(ADDR3, DENOM);

    suite.assert_pending_rewards(ADDR1, DENOM, 0);
    suite.assert_pending_rewards(ADDR2, DENOM, 0);
    suite.assert_pending_rewards(ADDR3, DENOM, 0);

    let contract = suite.distribution_contract.clone();

    // for 100k blocks there were no members so some rewards are remaining in the contract.
    let contract_token_balance = suite.get_balance_native(contract.clone(), DENOM);
    assert!(contract_token_balance > 0);
}

#[test]
#[should_panic(expected = "Invalid funds")]
fn test_fund_multiple_denoms() {
    let mut suite = SuiteBuilder::base(super::suite::DaoType::Native).build();

    let alt_coin = coin(100_000_000, ALT_DENOM);
    let coin = coin(100_000_000, DENOM);
    suite.mint_native_coin(alt_coin.clone(), OWNER);
    suite.mint_native_coin(coin.clone(), OWNER);
    let hook_caller = suite.staking_addr.to_string();
    suite.register_reward_denom(
        RewardsConfig {
            amount: 1000,
            denom: cw20::UncheckedDenom::Native(ALT_DENOM.to_string()),
            duration: Duration::Height(100),
            destination: None,
        },
        &hook_caller,
    );

    suite
        .app
        .borrow_mut()
        .execute_contract(
            Addr::unchecked(OWNER),
            suite.distribution_contract.clone(),
            &ExecuteMsg::Fund {},
            &[coin, alt_coin],
        )
        .unwrap();
}

#[test]
#[should_panic(expected = "unknown variant `not_the_fund: {}`")]
fn test_fund_cw20_with_invalid_cw20_receive_msg() {
    // attempting to fund a non-registered cw20 token should error
    let mut suite = SuiteBuilder::base(super::suite::DaoType::CW20).build();

    let unregistered_cw20_coin = Cw20Coin {
        address: ADDR1.to_string(),
        amount: Uint128::new(1_000_000),
    };

    let new_cw20_mint = suite.mint_cw20_coin(unregistered_cw20_coin.clone(), ADDR1, "newcoin");
    println!("[FUNDING EVENT] cw20 funding: {}", unregistered_cw20_coin);

    let fund_sub_msg = to_json_binary(&"not_the_fund: {}").unwrap();
    suite
        .app
        .execute_contract(
            Addr::unchecked(ADDR1),
            new_cw20_mint.clone(),
            &cw20::Cw20ExecuteMsg::Send {
                contract: suite.distribution_contract.to_string(),
                amount: unregistered_cw20_coin.amount,
                msg: fund_sub_msg,
            },
            &[],
        )
        .unwrap();
}

#[test]
#[should_panic]
fn test_fund_invalid_cw20_denom() {
    // attempting to fund a non-registered cw20 token should error
    let mut suite = SuiteBuilder::base(super::suite::DaoType::CW20).build();

    let unregistered_cw20_coin = Cw20Coin {
        address: ADDR1.to_string(),
        amount: Uint128::new(1_000_000),
    };

    println!("attempting to fund the distributor contract with unregistered cw20 coin");
    suite.fund_distributor_cw20(unregistered_cw20_coin);
}

#[test]
#[should_panic(expected = "Reward period already finished")]
fn test_shutdown_finished_rewards_period() {
    let mut suite = SuiteBuilder::base(super::suite::DaoType::Native).build();

    // skip to expiration
    suite.skip_blocks(2_000_000);

    suite.shutdown_denom_distribution(DENOM);
}

#[test]
fn test_shutdown_alternative_destination_address() {
    let subdao_addr = "some_subdao_maybe".to_string();
    let mut suite = SuiteBuilder::base(super::suite::DaoType::Native)
        .with_withdraw_destination(Some(subdao_addr.to_string()))
        .build();

    // skip 1/10th of the time
    suite.skip_blocks(100_000);

    suite.assert_pending_rewards(ADDR1, DENOM, 5_000_000);
    suite.assert_pending_rewards(ADDR2, DENOM, 2_500_000);
    suite.assert_pending_rewards(ADDR3, DENOM, 2_500_000);

    // user 1 and 2 claim their rewards
    suite.claim_rewards(ADDR1, DENOM);
    suite.claim_rewards(ADDR2, DENOM);

    // user 2 unstakes
    suite.unstake_native_tokens(ADDR2, 50);

    suite.skip_blocks(100_000);

    let distribution_contract = suite.distribution_contract.to_string();

    suite.assert_native_balance(subdao_addr.as_str(), DENOM, 0);
    let pre_shutdown_distributor_balance =
        suite.get_balance_native(distribution_contract.clone(), DENOM);

    suite.shutdown_denom_distribution(DENOM);

    let post_shutdown_distributor_balance =
        suite.get_balance_native(distribution_contract.clone(), DENOM);
    let post_shutdown_subdao_balance = suite.get_balance_native(subdao_addr.to_string(), DENOM);

    // after shutdown the balance of the subdao should be the same
    // as pre-shutdown-distributor-bal minus post-shutdown-distributor-bal
    assert_eq!(
        pre_shutdown_distributor_balance - post_shutdown_distributor_balance,
        post_shutdown_subdao_balance
    );
}

#[test]
fn test_shutdown_block_based() {
    let mut suite = SuiteBuilder::base(super::suite::DaoType::Native).build();

    // skip 1/10th of the time
    suite.skip_blocks(100_000);

    suite.assert_pending_rewards(ADDR1, DENOM, 5_000_000);
    suite.assert_pending_rewards(ADDR2, DENOM, 2_500_000);
    suite.assert_pending_rewards(ADDR3, DENOM, 2_500_000);

    // user 1 and 2 claim their rewards
    suite.claim_rewards(ADDR1, DENOM);
    suite.claim_rewards(ADDR2, DENOM);

    // user 2 unstakes
    suite.unstake_native_tokens(ADDR2, 50);

    suite.skip_blocks(100_000);

    let distribution_contract = suite.distribution_contract.to_string();

    let pre_shutdown_distributor_balance =
        suite.get_balance_native(distribution_contract.clone(), DENOM);

    suite.assert_native_balance(suite.owner.clone().unwrap().as_str(), DENOM, 0);
    suite.shutdown_denom_distribution(DENOM);

    let post_shutdown_distributor_balance =
        suite.get_balance_native(distribution_contract.clone(), DENOM);
    let post_shutdown_owner_balance = suite.get_balance_native(suite.owner.clone().unwrap(), DENOM);

    // after shutdown the balance of the owner should be the same
    // as pre-shutdown-distributor-bal minus post-shutdown-distributor-bal
    assert_eq!(
        pre_shutdown_distributor_balance - post_shutdown_distributor_balance,
        post_shutdown_owner_balance
    );

    suite.skip_blocks(100_000);

    // we assert that pending rewards did not change
    suite.assert_pending_rewards(ADDR1, DENOM, 6_666_666);
    suite.assert_pending_rewards(ADDR2, DENOM, 0);
    suite.assert_pending_rewards(ADDR3, DENOM, 5_833_333);

    // user 1 can claim their rewards
    suite.claim_rewards(ADDR1, DENOM);
    suite.assert_pending_rewards(ADDR1, DENOM, 0);
    suite.assert_native_balance(ADDR1, DENOM, 11_666_666);

    // user 3 can unstake and claim their rewards
    suite.unstake_native_tokens(ADDR3, 50);
    suite.skip_blocks(100_000);
    suite.assert_native_balance(ADDR3, DENOM, 50);
    suite.claim_rewards(ADDR3, DENOM);
    suite.assert_pending_rewards(ADDR3, DENOM, 0);
    suite.assert_native_balance(ADDR3, DENOM, 5_833_333 + 50);

    // TODO: fix this rug of 1 udenom by the distribution contract
    suite.assert_native_balance(&distribution_contract, DENOM, 1);
}

#[test]
fn test_shutdown_time_based() {
    let mut suite = SuiteBuilder::base(super::suite::DaoType::Native)
        .with_rewards_config(RewardsConfig {
            amount: 1_000,
            denom: UncheckedDenom::Native(DENOM.to_string()),
            duration: Duration::Time(10),
            destination: None,
        })
        .build();

    // skip 1/10th of the time
    suite.skip_seconds(100_000);

    suite.assert_pending_rewards(ADDR1, DENOM, 5_000_000);
    suite.assert_pending_rewards(ADDR2, DENOM, 2_500_000);
    suite.assert_pending_rewards(ADDR3, DENOM, 2_500_000);

    // user 1 and 2 claim their rewards
    suite.claim_rewards(ADDR1, DENOM);
    suite.claim_rewards(ADDR2, DENOM);

    // user 2 unstakes
    suite.unstake_native_tokens(ADDR2, 50);

    suite.skip_seconds(100_000);

    let distribution_contract = suite.distribution_contract.to_string();

    let pre_shutdown_distributor_balance =
        suite.get_balance_native(distribution_contract.clone(), DENOM);

    suite.assert_native_balance(suite.owner.clone().unwrap().as_str(), DENOM, 0);
    suite.shutdown_denom_distribution(DENOM);

    let post_shutdown_distributor_balance =
        suite.get_balance_native(distribution_contract.clone(), DENOM);
    let post_shutdown_owner_balance = suite.get_balance_native(suite.owner.clone().unwrap(), DENOM);

    // after shutdown the balance of the owner should be the same
    // as pre-shutdown-distributor-bal minus post-shutdown-distributor-bal
    assert_eq!(
        pre_shutdown_distributor_balance - post_shutdown_distributor_balance,
        post_shutdown_owner_balance
    );

    suite.skip_seconds(100_000);

    // we assert that pending rewards did not change
    suite.assert_pending_rewards(ADDR1, DENOM, 6_666_666);
    suite.assert_pending_rewards(ADDR2, DENOM, 0);
    suite.assert_pending_rewards(ADDR3, DENOM, 5_833_333);

    // user 1 can claim their rewards
    suite.claim_rewards(ADDR1, DENOM);
    suite.assert_pending_rewards(ADDR1, DENOM, 0);
    suite.assert_native_balance(ADDR1, DENOM, 11_666_666);

    // user 3 can unstake and claim their rewards
    suite.unstake_native_tokens(ADDR3, 50);
    suite.skip_seconds(100_000);
    suite.assert_native_balance(ADDR3, DENOM, 50);
    suite.claim_rewards(ADDR3, DENOM);
    suite.assert_pending_rewards(ADDR3, DENOM, 0);
    suite.assert_native_balance(ADDR3, DENOM, 5_833_333 + 50);

    // TODO: fix this rug of 1 udenom by the distribution contract
    suite.assert_native_balance(&distribution_contract, DENOM, 1);
}

#[test]
#[should_panic(expected = "Caller is not the contract's current owner")]
fn test_shudown_unauthorized() {
    let mut suite = SuiteBuilder::base(super::suite::DaoType::Native).build();

    // skip 1/10th of the time
    suite.skip_blocks(100_000);

    suite
        .app
        .borrow_mut()
        .execute_contract(
            Addr::unchecked(ADDR1),
            suite.distribution_contract.clone(),
            &ExecuteMsg::Shutdown {
                denom: DENOM.to_string(),
            },
            &[],
        )
        .unwrap();
}

#[test]
#[should_panic]
fn test_shutdown_unregistered_denom() {
    let mut suite = SuiteBuilder::base(super::suite::DaoType::Native).build();

    suite.skip_blocks(100_000);

    suite.shutdown_denom_distribution("not-the-denom");
}

#[test]
#[should_panic(expected = "Denom already registered")]
fn test_register_duplicate_denom() {
    let mut suite = SuiteBuilder::base(super::suite::DaoType::Native).build();

    let hook_caller = suite.staking_addr.to_string();
    let reward_config = RewardsConfig {
        amount: 1000,
        denom: cw20::UncheckedDenom::Native(DENOM.to_string()),
        duration: Duration::Height(100),
        destination: None,
    };
    suite.register_reward_denom(reward_config, &hook_caller);
}

#[test]
#[should_panic]
fn test_fund_invalid_native_denom() {
    let mut suite = SuiteBuilder::base(super::suite::DaoType::Native).build();

    // skip 1/10th of the time
    suite.skip_blocks(100_000);

    suite.mint_native_coin(coin(100_000_000, ALT_DENOM), OWNER);
    suite
        .app
        .borrow_mut()
        .execute_contract(
            Addr::unchecked(OWNER),
            suite.distribution_contract.clone(),
            &ExecuteMsg::Fund {},
            &[coin(100_000_000, ALT_DENOM)],
        )
        .unwrap();
}

#[test]
fn test_fund_unauthorized() {
    let mut suite = SuiteBuilder::base(super::suite::DaoType::Native).build();

    // skip 1/10th of the time
    suite.skip_blocks(100_000);

    suite.mint_native_coin(coin(100_000_000, DENOM), ADDR1);
    suite
        .app
        .borrow_mut()
        .execute_contract(
            Addr::unchecked(ADDR1),
            suite.distribution_contract.clone(),
            &ExecuteMsg::Fund {},
            &[coin(100_000_000, DENOM)],
        )
        .unwrap();
}

#[test]
fn test_fund_native_block_based_post_expiration() {
    let mut suite = SuiteBuilder::base(super::suite::DaoType::Native).build();

    let started_at = Expiration::AtHeight(0);
    let funded_blocks = 1_000_000;
    let expiration_date = Expiration::AtHeight(funded_blocks);
    suite.assert_amount(1_000);
    suite.assert_ends_at(expiration_date);
    suite.assert_started_at(started_at);
    suite.assert_duration(10);

    // skip 1/10th of the time
    suite.skip_blocks(100_000);

    suite.assert_pending_rewards(ADDR1, DENOM, 5_000_000);
    suite.assert_pending_rewards(ADDR2, DENOM, 2_500_000);
    suite.assert_pending_rewards(ADDR3, DENOM, 2_500_000);

    // ADDR2 unstake their stake
    suite.unstake_native_tokens(ADDR2, 50);

    // addr3 claims their rewards
    suite.claim_rewards(ADDR3, DENOM);

    // skip to 100_000 blocks past the expiration
    suite.skip_blocks(1_000_000);

    suite.assert_pending_rewards(ADDR1, DENOM, 65_000_000);
    suite.assert_pending_rewards(ADDR2, DENOM, 2_500_000);
    suite.assert_pending_rewards(ADDR3, DENOM, 30_000_000);

    suite.assert_ends_at(expiration_date);
    suite.assert_started_at(started_at);

    // we fund the distributor with the same amount of coins as
    // during setup, meaning that the rewards distribution duration
    // should be the same.
    suite.fund_distributor_native(coin(100_000_000, DENOM));

    let current_block = suite.app.block_info();

    // funding after the reward period had expired should
    // reset the start date to that of the funding.
    suite.assert_started_at(Expiration::AtHeight(current_block.height));

    // funding after the reward period had expired should
    // set the distribution expiration to the funded duration
    // after current block
    suite.assert_ends_at(Expiration::AtHeight(current_block.height + funded_blocks));
}

#[test]
fn test_fund_cw20_time_based_post_expiration() {
    let mut suite = SuiteBuilder::base(super::suite::DaoType::CW20)
        .with_rewards_config(RewardsConfig {
            amount: 1_000,
            denom: UncheckedDenom::Cw20(DENOM.to_string()),
            duration: Duration::Time(10),
            destination: None,
        })
        .build();

    let started_at = Expiration::AtTime(Timestamp::from_seconds(0));
    let funded_timestamp = Timestamp::from_seconds(1_000_000);
    let expiration_date = Expiration::AtTime(funded_timestamp);
    suite.assert_amount(1_000);
    suite.assert_ends_at(expiration_date);
    suite.assert_started_at(started_at);
    suite.assert_duration(10);

    // skip 1/10th of the time
    suite.skip_seconds(100_000);

    suite.assert_pending_rewards(ADDR1, DENOM, 5_000_000);
    suite.assert_pending_rewards(ADDR2, DENOM, 2_500_000);
    suite.assert_pending_rewards(ADDR3, DENOM, 2_500_000);

    // ADDR2 unstake their stake
    suite.unstake_cw20_tokens(50, ADDR2);

    // addr3 claims their rewards
    suite.claim_rewards(ADDR3, suite.reward_denom.clone().as_str());
    suite.assert_cw20_balance(ADDR3, 2_500_000);

    // skip to 100_000 blocks past the expiration
    suite.skip_seconds(1_000_000);

    suite.assert_pending_rewards(ADDR1, DENOM, 65_000_000);
    suite.assert_pending_rewards(ADDR2, DENOM, 2_500_000);
    suite.assert_pending_rewards(ADDR3, DENOM, 30_000_000);

    suite.assert_ends_at(expiration_date);
    suite.assert_started_at(started_at);

    // we fund the distributor with the same amount of coins as
    // during setup, meaning that the rewards distribution duration
    // should be the same.
    let funding_denom = Cw20Coin {
        address: suite.reward_denom.to_string(),
        amount: Uint128::new(100_000_000),
    };

    suite.fund_distributor_cw20(funding_denom.clone());

    let current_block = suite.app.block_info();

    // funding after the reward period had expired should
    // reset the start date to that of the funding.
    suite.assert_started_at(Expiration::AtTime(current_block.time));

    // funding after the reward period had expired should
    // set the distribution expiration to the funded duration
    // after current block
    suite.assert_ends_at(Expiration::AtTime(
        current_block.time.plus_seconds(funded_timestamp.seconds()),
    ));
}

#[test]
fn test_fund_cw20_time_based_pre_expiration() {
    let mut suite = SuiteBuilder::base(super::suite::DaoType::CW20)
        .with_rewards_config(RewardsConfig {
            amount: 1_000,
            denom: UncheckedDenom::Cw20(DENOM.to_string()),
            duration: Duration::Time(10),
            destination: None,
        })
        .build();

    let started_at = Expiration::AtTime(Timestamp::from_seconds(0));
    let funded_timestamp = Timestamp::from_seconds(1_000_000);
    let expiration_date = Expiration::AtTime(funded_timestamp);
    suite.assert_amount(1_000);
    suite.assert_ends_at(expiration_date);
    suite.assert_started_at(started_at);
    suite.assert_duration(10);

    // skip 1/10th of the time
    suite.skip_seconds(100_000);

    suite.assert_pending_rewards(ADDR1, DENOM, 5_000_000);
    suite.assert_pending_rewards(ADDR2, DENOM, 2_500_000);
    suite.assert_pending_rewards(ADDR3, DENOM, 2_500_000);

    // ADDR2 unstake their stake
    suite.unstake_cw20_tokens(50, ADDR2);

    // addr3 claims their rewards
    suite.claim_rewards(ADDR3, suite.reward_denom.clone().as_str());

    // skip to 100_000 blocks before the expiration
    suite.skip_seconds(800_000);

    suite.assert_pending_rewards(ADDR1, DENOM, 58_333_333);
    suite.assert_pending_rewards(ADDR2, DENOM, 2_500_000);
    suite.assert_pending_rewards(ADDR3, DENOM, 26_666_666);

    suite.assert_ends_at(expiration_date);
    suite.assert_started_at(started_at);

    // we fund the distributor with the same amount of coins as
    // during setup, meaning that the rewards distribution duration
    // should be the same.
    let funding_denom = Cw20Coin {
        address: suite.reward_denom.to_string(),
        amount: Uint128::new(100_000_000),
    };
    suite.fund_distributor_cw20(funding_denom.clone());

    // funding before the reward period expires should
    // not reset the existing rewards cycle
    suite.assert_started_at(started_at);

    // funding before the reward period expires should
    // extend the current distribution expiration by the
    // newly funded duration
    suite.assert_ends_at(Expiration::AtTime(Timestamp::from_seconds(
        funded_timestamp.seconds() * 2,
    )));
}

#[test]
fn test_fund_native_height_based_pre_expiration() {
    let mut suite = SuiteBuilder::base(super::suite::DaoType::Native).build();

    let started_at = Expiration::AtHeight(0);
    let funded_blocks = 1_000_000;
    let expiration_date = Expiration::AtHeight(funded_blocks);
    suite.assert_amount(1_000);
    suite.assert_ends_at(expiration_date);
    suite.assert_started_at(started_at);
    suite.assert_duration(10);

    // skip 1/10th of the time
    suite.skip_blocks(100_000);

    suite.assert_pending_rewards(ADDR1, DENOM, 5_000_000);
    suite.assert_pending_rewards(ADDR2, DENOM, 2_500_000);
    suite.assert_pending_rewards(ADDR3, DENOM, 2_500_000);

    // ADDR2 unstake their stake
    suite.unstake_native_tokens(ADDR2, 50);

    // addr3 claims their rewards
    suite.claim_rewards(ADDR3, DENOM);

    // skip to 100_000 blocks before the expiration
    suite.skip_blocks(800_000);

    suite.assert_pending_rewards(ADDR1, DENOM, 58_333_333);
    suite.assert_pending_rewards(ADDR2, DENOM, 2_500_000);
    suite.assert_pending_rewards(ADDR3, DENOM, 26_666_666);

    suite.assert_ends_at(expiration_date);
    suite.assert_started_at(started_at);

    // we fund the distributor with the same amount of coins as
    // during setup, meaning that the rewards distribution duration
    // should be the same.
    suite.fund_distributor_native(coin(100_000_000, DENOM));

    // funding before the reward period expires should
    // not reset the existing rewards cycle
    suite.assert_started_at(started_at);

    // funding before the reward period expires should
    // extend the current distribution expiration by the
    // newly funded duration
    suite.assert_ends_at(Expiration::AtHeight(funded_blocks * 2));
}

#[test]
fn test_native_dao_rewards_entry_edge_case() {
    let mut suite = SuiteBuilder::base(super::suite::DaoType::Native).build();

    // we start with the following staking power split:
    // [ADDR1: 100, ADDR2: 50, ADDR3: 50], or [ADDR1: 50%, ADDR2: 25%, ADDR3: 25%
    suite.assert_amount(1_000);
    suite.assert_ends_at(Expiration::AtHeight(1_000_000));
    suite.assert_duration(10);

    // skip 1/10th of the time
    suite.skip_blocks(100_000);

    // ADDR1 stakes additional 100 tokens, bringing the new staking power split to
    // [ADDR1: 200, ADDR2: 50, ADDR3: 50], or [ADDR1: 66.6%, ADDR2: 16.6%, ADDR3: 16.6%]
    // this means that per 100_000 blocks, ADDR1 should receive 6_666_666, while
    // ADDR2 and ADDR3 should receive 1_666_666 each.
    suite.mint_native_coin(coin(100, DENOM), ADDR1);
    println!("staking native coins\n");
    suite.stake_native_tokens(ADDR1, 100);

    // rewards here should not be affected by the new stake,
    suite.assert_pending_rewards(ADDR1, DENOM, 5_000_000);
    suite.assert_pending_rewards(ADDR2, DENOM, 2_500_000);
    suite.assert_pending_rewards(ADDR3, DENOM, 2_500_000);

    // skip 1/10th of the time
    suite.skip_blocks(100_000);

    // here we should see the new stake affecting the rewards split.
    suite.assert_pending_rewards(ADDR1, DENOM, 5_000_000 + 6_666_666);
    suite.assert_pending_rewards(ADDR2, DENOM, 2_500_000 + 1_666_666);
    suite.assert_pending_rewards(ADDR3, DENOM, 2_500_000 + 1_666_666);

    // ADDR1 claims rewards
    suite.claim_rewards(ADDR1, DENOM);
    suite.assert_native_balance(ADDR1, DENOM, 5_000_000 + 6_666_666);
    suite.assert_pending_rewards(ADDR1, DENOM, 0);

    // ADDR2 and ADDR3 unstake their stake
    // new voting power split is [ADDR1: 100%, ADDR2: 0%, ADDR3: 0%]
    suite.unstake_native_tokens(ADDR2, 50);
    suite.unstake_native_tokens(ADDR3, 50);

    // we assert that by unstaking, ADDR2 and ADDR3 do not forfeit their earned but unclaimed rewards
    suite.assert_pending_rewards(ADDR2, DENOM, 2_500_000 + 1_666_666);
    suite.assert_pending_rewards(ADDR3, DENOM, 2_500_000 + 1_666_666);

    // skip a block and assert that nothing changes
    suite.skip_blocks(1);
    suite.assert_pending_rewards(ADDR2, DENOM, 2_500_000 + 1_666_666);
    suite.assert_pending_rewards(ADDR3, DENOM, 2_500_000 + 1_666_666);

    // skip the remaining blocks to reach 1/10th of the time
    suite.skip_blocks(99_999);

    // because ADDR2 and ADDR3 are not staking, ADDR1 receives all the rewards.
    // ADDR2 and ADDR3 should have the same amount of pending rewards as before.
    suite.assert_pending_rewards(ADDR1, DENOM, 10_000_000);
    suite.assert_pending_rewards(ADDR2, DENOM, 2_500_000 + 1_666_666);
    suite.assert_pending_rewards(ADDR3, DENOM, 2_500_000 + 1_666_666);

    // ADDR2 and ADDR3 wake up, claim and restake their rewards
    suite.claim_rewards(ADDR2, DENOM);
    suite.claim_rewards(ADDR3, DENOM);

    let addr1_balance = suite.get_balance_native(ADDR1, DENOM);
    let addr2_balance = suite.get_balance_native(ADDR2, DENOM);

    suite.stake_native_tokens(ADDR1, addr1_balance);
    suite.stake_native_tokens(ADDR2, addr2_balance);
}

#[test]
fn test_update_owner() {
    let mut suite = SuiteBuilder::base(super::suite::DaoType::Native).build();

    let new_owner = "new_owner";
    suite.update_owner(new_owner);

    let owner = suite.get_owner().to_string();
    assert_eq!(owner, new_owner);
}
