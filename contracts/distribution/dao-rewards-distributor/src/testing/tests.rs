use std::borrow::BorrowMut;

use cosmwasm_std::{coin, Addr};
use cw20::Expiration;
use cw_multi_test::Executor;

use crate::{
    msg::ExecuteMsg,
    testing::{ADDR1, ADDR2, ADDR3, DENOM},
};

use super::suite::SuiteBuilder;

// By default, the tests are set up to distribute rewards over 1_000_000 units of time.
// Over that time, 100_000_000 token rewards will be distributed.

#[test]
fn test_cw20_dao_native_rewards_block_height_based() {
    let mut suite = SuiteBuilder::base(super::suite::DaoType::CW20).build();

    suite.assert_reward_rate_emission(1_000);
    suite.assert_distribution_expiration(Expiration::AtHeight(1_000_000));
    suite.assert_reward_rate_time(10);

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
    println!("Remaining time: {:?}", remaining_time);

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

    suite.assert_reward_rate_emission(1_000);
    suite.assert_distribution_expiration(Expiration::AtHeight(1_000_000));
    suite.assert_reward_rate_time(10);

    // skip 1/10th of the time
    suite.skip_blocks(100_000);

    suite.assert_pending_rewards(ADDR1, DENOM, 50_000_000);
    suite.assert_pending_rewards(ADDR2, DENOM, 25_000_000);
    suite.assert_pending_rewards(ADDR3, DENOM, 25_000_000);

    // skip 1/10th of the time
    suite.skip_blocks(100_000);

    suite.assert_pending_rewards(ADDR1, DENOM, 100_000_000);
    suite.assert_pending_rewards(ADDR2, DENOM, 50_000_000);
    suite.assert_pending_rewards(ADDR3, DENOM, 50_000_000);

    // ADDR1 claims rewards
    suite.claim_rewards(ADDR1, DENOM);
    suite.assert_native_balance(ADDR1, DENOM, 100_000_000);
    suite.assert_pending_rewards(ADDR1, DENOM, 0);

    // ADDR2 and ADDR3 unstake their nfts
    suite.unstake_nft(ADDR2, 3);
    suite.unstake_nft(ADDR3, 4);

    // skip 1/10th of the time
    suite.skip_blocks(100_000);

    // because ADDR2 and ADDR3 are not staking, ADDR1 receives all the rewards.
    // ADDR2 and ADDR3 should have the same amount of pending rewards as before.
    suite.assert_pending_rewards(ADDR1, DENOM, 100_000_000);
    suite.assert_pending_rewards(ADDR2, DENOM, 50_000_000);
    suite.assert_pending_rewards(ADDR3, DENOM, 50_000_000);

    // ADDR2 and ADDR3 wake up, claim and restake their nfts
    suite.claim_rewards(ADDR2, DENOM);
    suite.claim_rewards(ADDR3, DENOM);

    suite.stake_nft(ADDR2, 3);
    suite.stake_nft(ADDR3, 4);
}

#[test]
fn test_native_dao_rewards() {
    let mut suite = SuiteBuilder::base(super::suite::DaoType::NATIVE).build();

    suite.assert_reward_rate_emission(1_000);
    suite.assert_distribution_expiration(Expiration::AtHeight(1_000_000));
    suite.assert_reward_rate_time(10);

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

    suite.assert_reward_rate_emission(1_000);
    suite.assert_distribution_expiration(Expiration::AtHeight(1_000_000));
    suite.assert_reward_rate_time(10);

    // skip 1/10th of the time
    suite.skip_blocks(100_000);

    suite.assert_pending_rewards(ADDR1, DENOM, 50_000_000);
    suite.assert_pending_rewards(ADDR2, DENOM, 25_000_000);
    suite.assert_pending_rewards(ADDR3, DENOM, 25_000_000);

    // skip 1/10th of the time
    suite.skip_blocks(100_000);

    suite.assert_pending_rewards(ADDR1, DENOM, 100_000_000);
    suite.assert_pending_rewards(ADDR2, DENOM, 50_000_000);
    suite.assert_pending_rewards(ADDR3, DENOM, 50_000_000);

    // ADDR1 claims rewards
    suite.claim_rewards(ADDR1, DENOM);
    suite.assert_native_balance(ADDR1, DENOM, 100_000_000);
    suite.assert_pending_rewards(ADDR1, DENOM, 0);
}

#[test]
fn test_fund_multiple_denoms() {
    unimplemented!()
}

#[test]
fn test_shutdown_happy() {
    let mut suite = SuiteBuilder::base(super::suite::DaoType::NATIVE).build();

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
#[should_panic(expected = "Caller is not the contract's current owner")]
fn test_shudown_unauthorized() {
    let mut suite = SuiteBuilder::base(super::suite::DaoType::NATIVE).build();

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
    let mut suite = SuiteBuilder::base(super::suite::DaoType::NATIVE).build();

    suite.skip_blocks(100_000);

    suite.shutdown_denom_distribution("not-the-denom");
}

#[test]
fn test_update_reward_duration() {
    unimplemented!()
}

#[test]
fn test_fund_invalid_cw20_denom() {
    unimplemented!()
}

#[test]
fn test_fund_invalid_native_denom() {
    unimplemented!()
}

#[test]
fn test_fund_unauthorized() {
    unimplemented!()
}

#[test]
fn test_fund_post_expiration() {
    let mut suite = SuiteBuilder::base(super::suite::DaoType::NATIVE).build();

    let start_date = Expiration::AtHeight(0);
    let funded_blocks = 1_000_000;
    let expiration_date = Expiration::AtHeight(funded_blocks);
    suite.assert_reward_rate_emission(1_000);
    suite.assert_distribution_expiration(expiration_date);
    suite.assert_period_start_date(start_date);
    suite.assert_reward_rate_time(10);

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

    suite.assert_distribution_expiration(expiration_date);
    suite.assert_period_start_date(start_date);

    // we fund the distributor with the same amount of coins as
    // during setup, meaning that the rewards distribution duration
    // should be the same.
    suite.fund_distributor_native(coin(100_000_000, DENOM));

    let current_block = suite.app.block_info();

    // funding after the reward period had expired should
    // reset the start date to that of the funding.
    suite.assert_period_start_date(Expiration::AtHeight(current_block.height));

    // funding after the reward period had expired should
    // set the distribution expiration to the funded duration
    // after current block
    suite
        .assert_distribution_expiration(Expiration::AtHeight(current_block.height + funded_blocks));
}

#[test]
fn test_fund_pre_expiration() {
    let mut suite = SuiteBuilder::base(super::suite::DaoType::NATIVE).build();

    let start_date = Expiration::AtHeight(0);
    let funded_blocks = 1_000_000;
    let expiration_date = Expiration::AtHeight(funded_blocks);
    suite.assert_reward_rate_emission(1_000);
    suite.assert_distribution_expiration(expiration_date);
    suite.assert_period_start_date(start_date);
    suite.assert_reward_rate_time(10);

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

    suite.assert_distribution_expiration(expiration_date);
    suite.assert_period_start_date(start_date);

    // we fund the distributor with the same amount of coins as
    // during setup, meaning that the rewards distribution duration
    // should be the same.
    suite.fund_distributor_native(coin(100_000_000, DENOM));

    // funding before the reward period expires should
    // not reset the existing rewards cycle
    suite.assert_period_start_date(start_date);

    // funding before the reward period expires should
    // extend the current distribution expiration by the
    // newly funded duration
    suite.assert_distribution_expiration(Expiration::AtHeight(funded_blocks * 2));
}
