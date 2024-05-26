use cw20::Expiration;

use crate::testing::{ADDR1, ADDR2, ADDR3, DENOM};

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

    // ADDR2 and ADDR3 unstake their stake
    suite.unstake_native_tokens(ADDR2, 50);
    suite.unstake_native_tokens(ADDR3, 50);

    // skip 1/10th of the time
    suite.skip_blocks(100_000);

    // because ADDR2 and ADDR3 are not staking, ADDR1 receives all the rewards.
    // ADDR2 and ADDR3 should have the same amount of pending rewards as before.
    suite.assert_pending_rewards(ADDR1, DENOM, 100_000_000);
    suite.assert_pending_rewards(ADDR2, DENOM, 50_000_000);
    suite.assert_pending_rewards(ADDR3, DENOM, 50_000_000);

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
fn test_shutdown() {
    unimplemented!()
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
    unimplemented!()
}

#[test]
fn test_fund_pre_expiration() {
    unimplemented!()
}

#[test]
fn test_shudown_unauthorized() {
    unimplemented!()
}

#[test]
fn test_shutdown_unregistered_denom() {
    unimplemented!()
}

#[test]
fn test_shutdown_active_distribution() {
    unimplemented!()
}
