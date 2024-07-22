use std::borrow::BorrowMut;

use cosmwasm_std::{coin, coins, to_json_binary, Addr, Timestamp};
use cosmwasm_std::{Uint128, Uint256};
use cw2::ContractVersion;
use cw20::{Cw20Coin, Expiration, UncheckedDenom};
use cw4::Member;
use cw_multi_test::Executor;
use cw_utils::Duration;
use dao_interface::voting::InfoResponse;

use crate::msg::{CreateMsg, FundMsg};
use crate::state::{EmissionRate, Epoch};
use crate::testing::native_setup::setup_native_token_test;
use crate::ContractError;
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
#[should_panic(expected = "Distribution not found with ID 3")]
fn test_fund_native_404() {
    let mut suite = SuiteBuilder::base(super::suite::DaoType::Native).build();

    let mint_coin = coin(100, DENOM);

    suite.mint_native(mint_coin.clone(), OWNER);
    suite.fund_native(3, mint_coin);
}

#[test]
#[should_panic(expected = "Distribution not found with ID 3")]
fn test_fund_cw20_404() {
    let mut suite = SuiteBuilder::base(super::suite::DaoType::CW20)
        .with_rewards_config(RewardsConfig {
            amount: 1_000,
            denom: UncheckedDenom::Cw20("irrelevant".to_string()),
            duration: Duration::Height(10),
            destination: None,
            continuous: true,
        })
        .build();

    let mint_cw20 = Cw20Coin {
        address: OWNER.to_string(),
        amount: Uint128::new(100),
    };

    let address = suite.mint_cw20(mint_cw20.clone(), "newcoin").to_string();

    suite.fund_cw20(
        3,
        Cw20Coin {
            address,
            amount: mint_cw20.amount,
        },
    );
}

#[test]
fn test_native_dao_rewards_update_reward_rate() {
    let mut suite = SuiteBuilder::base(super::suite::DaoType::Native).build();

    suite.assert_amount(1_000);
    suite.assert_ends_at(Expiration::AtHeight(1_000_000));
    suite.assert_duration(10);

    // skip 1/10th of the time
    suite.skip_blocks(100_000);

    suite.assert_pending_rewards(ADDR1, 1, 5_000_000);
    suite.assert_pending_rewards(ADDR2, 1, 2_500_000);
    suite.assert_pending_rewards(ADDR3, 1, 2_500_000);

    // skip 1/10th of the time
    suite.skip_blocks(100_000);

    suite.assert_pending_rewards(ADDR1, 1, 10_000_000);
    suite.assert_pending_rewards(ADDR2, 1, 5_000_000);
    suite.assert_pending_rewards(ADDR3, 1, 5_000_000);

    // ADDR1 claims rewards
    suite.claim_rewards(ADDR1, 1);
    suite.assert_pending_rewards(ADDR1, 1, 0);

    // set the rewards rate to half of the current one
    // now there will be 5_000_000 tokens distributed over 100_000 blocks
    suite.update_emission_rate(1, Duration::Height(10), 500);

    // skip 1/10th of the time
    suite.skip_blocks(100_000);

    suite.assert_pending_rewards(ADDR1, 1, 2_500_000);
    suite.assert_pending_rewards(ADDR2, 1, 6_250_000);
    suite.assert_pending_rewards(ADDR3, 1, 6_250_000);

    // double the rewards rate
    // now there will be 10_000_000 tokens distributed over 100_000 blocks
    suite.update_emission_rate(1, Duration::Height(10), 1_000);

    // skip 1/10th of the time
    suite.skip_blocks(100_000);

    suite.assert_pending_rewards(ADDR1, 1, 7_500_000);
    suite.assert_pending_rewards(ADDR2, 1, 8_750_000);
    suite.assert_pending_rewards(ADDR3, 1, 8_750_000);

    // skip 2/10ths of the time
    suite.skip_blocks(200_000);

    suite.assert_pending_rewards(ADDR1, 1, 17_500_000);
    suite.assert_pending_rewards(ADDR2, 1, 13_750_000);
    suite.assert_pending_rewards(ADDR3, 1, 13_750_000);

    // pause the rewards distribution
    suite.pause_emission(1);

    // skip 1/10th of the time
    suite.skip_blocks(100_000);

    // assert no pending rewards changed
    suite.assert_pending_rewards(ADDR1, 1, 17_500_000);
    suite.assert_pending_rewards(ADDR2, 1, 13_750_000);
    suite.assert_pending_rewards(ADDR3, 1, 13_750_000);

    // assert ADDR1 pre-claim balance
    suite.assert_native_balance(ADDR1, DENOM, 10_000_000);
    // ADDR1 claims their rewards
    suite.claim_rewards(ADDR1, 1);
    // assert ADDR1 post-claim balance to be pre-claim + pending
    suite.assert_native_balance(ADDR1, DENOM, 10_000_000 + 17_500_000);
    // assert ADDR1 is now entitled to 0 pending rewards
    suite.assert_pending_rewards(ADDR1, 1, 0);

    // user 2 unstakes their stake
    suite.unstake_native_tokens(ADDR2, 50);

    // skip 1/10th of the time
    suite.skip_blocks(100_000);

    // only the ADDR1 pending rewards should have changed
    suite.assert_pending_rewards(ADDR1, 1, 0);
    suite.assert_pending_rewards(ADDR2, 1, 13_750_000);
    suite.assert_pending_rewards(ADDR3, 1, 13_750_000);

    // ADDR2 claims their rewards (has 50 to begin with as they unstaked)
    suite.assert_native_balance(ADDR2, DENOM, 50);
    suite.claim_rewards(ADDR2, 1);
    // assert ADDR2 post-claim balance to be pre-claim + pending and has 0 pending rewards
    suite.assert_native_balance(ADDR2, DENOM, 13_750_000 + 50);
    suite.assert_pending_rewards(ADDR2, 1, 0);

    // update the reward rate back to 1_000 / 10blocks
    // this should now distribute 10_000_000 tokens over 100_000 blocks
    // between ADDR1 (2/3rds) and ADDR3 (1/3rd)
    suite.update_emission_rate(1, Duration::Height(10), 1000);

    // update with the same rate does nothing
    suite.update_emission_rate(1, Duration::Height(10), 1000);

    // skip 1/10th of the time
    suite.skip_blocks(100_000);

    // assert that rewards are being distributed at the expected rate
    suite.assert_pending_rewards(ADDR1, 1, 6_666_666);
    suite.assert_pending_rewards(ADDR2, 1, 0);
    suite.assert_pending_rewards(ADDR3, 1, 13_750_000 + 3_333_333);

    // ADDR3 claims their rewards
    suite.assert_native_balance(ADDR3, DENOM, 0);
    suite.claim_rewards(ADDR3, 1);
    suite.assert_pending_rewards(ADDR3, 1, 0);
    suite.assert_native_balance(ADDR3, DENOM, 13_750_000 + 3_333_333);

    // skip 1/10th of the time
    suite.skip_blocks(100_000);

    suite.assert_pending_rewards(ADDR1, 1, 6_666_666 + 6_666_666 + 1);
    suite.assert_pending_rewards(ADDR2, 1, 0);
    suite.assert_pending_rewards(ADDR3, 1, 3_333_333);

    // claim everything so that there are 0 pending rewards
    suite.claim_rewards(ADDR3, 1);
    suite.claim_rewards(ADDR1, 1);

    suite.assert_pending_rewards(ADDR1, 1, 0);
    suite.assert_pending_rewards(ADDR2, 1, 0);
    suite.assert_pending_rewards(ADDR3, 1, 0);

    // update the rewards rate to 40_000_000 per 100_000 blocks.
    // split is still 2/3rds to ADDR1 and 1/3rd to ADDR3
    suite.update_emission_rate(1, Duration::Height(10), 4000);
    suite.assert_ends_at(Expiration::AtHeight(1_062_500));

    suite.skip_blocks(50_000); // allocates 20_000_000 tokens

    let addr1_pending = 20_000_000 * 2 / 3;
    let addr3_pending = 20_000_000 / 3;
    suite.assert_pending_rewards(ADDR1, 1, addr1_pending);
    suite.assert_pending_rewards(ADDR2, 1, 0);
    suite.assert_pending_rewards(ADDR3, 1, addr3_pending);

    // ADDR2 wakes up to the increased staking rate and stakes 50 tokens
    // this brings new split to: [ADDR1: 50%, ADDR2: 25%, ADDR3: 25%]
    suite.stake_native_tokens(ADDR2, 50);

    suite.skip_blocks(10_000); // allocates 4_000_000 tokens

    suite.assert_pending_rewards(ADDR1, 1, addr1_pending + 4_000_000 * 2 / 4);
    suite.assert_pending_rewards(ADDR2, 1, 4_000_000 / 4);
    suite.assert_pending_rewards(ADDR3, 1, addr3_pending + 4_000_000 / 4);

    suite.claim_rewards(ADDR1, 1);
    suite.claim_rewards(ADDR3, 1);
    let addr1_pending = 0;
    let addr3_pending = 0;
    suite.skip_blocks(10_000); // skips from 1,060,000 to 1,070,000, and the end is 1,062,500, so this allocates only 1_000_000 tokens instead of 4_000_000

    suite.assert_pending_rewards(ADDR1, 1, addr1_pending + 1_000_000 * 2 / 4);
    suite.assert_pending_rewards(ADDR2, 1, 4_000_000 / 4 + 1_000_000 / 4);
    suite.assert_pending_rewards(ADDR3, 1, addr3_pending + 1_000_000 / 4);

    suite.claim_rewards(ADDR2, 1);

    // TODO: there's a few denoms remaining here, ensure such cases are handled properly
    let remaining_rewards = suite.get_balance_native(suite.distribution_contract.clone(), DENOM);
    println!("Remaining rewards: {}", remaining_rewards);
}

#[test]
fn test_native_dao_rewards_reward_rate_switch_unit() {
    let mut suite = SuiteBuilder::base(super::suite::DaoType::Native)
        .with_rewards_config(RewardsConfig {
            amount: 1_000,
            denom: UncheckedDenom::Native(DENOM.to_string()),
            duration: Duration::Height(10),
            destination: None,
            continuous: true,
        })
        .build();

    suite.assert_amount(1_000);
    suite.assert_ends_at(Expiration::AtHeight(1_000_000));
    suite.assert_duration(10);

    // skip 1/10th of the time
    suite.skip_blocks(100_000);

    suite.assert_pending_rewards(ADDR1, 1, 5_000_000);
    suite.assert_pending_rewards(ADDR2, 1, 2_500_000);
    suite.assert_pending_rewards(ADDR3, 1, 2_500_000);

    // skip 1/10th of the time
    suite.skip_blocks(100_000);

    suite.assert_pending_rewards(ADDR1, 1, 10_000_000);
    suite.assert_pending_rewards(ADDR2, 1, 5_000_000);
    suite.assert_pending_rewards(ADDR3, 1, 5_000_000);

    // ADDR1 claims rewards
    suite.claim_rewards(ADDR1, 1);
    suite.assert_pending_rewards(ADDR1, 1, 0);

    // set the rewards rate to time-based rewards
    suite.update_emission_rate(1, Duration::Time(10), 500);

    // skip 1/10th of the time
    suite.skip_seconds(100_000);

    suite.assert_pending_rewards(ADDR1, 1, 2_500_000);
    suite.assert_pending_rewards(ADDR2, 1, 6_250_000);
    suite.assert_pending_rewards(ADDR3, 1, 6_250_000);

    // double the rewards rate
    // now there will be 10_000_000 tokens distributed over 100_000 seconds
    suite.update_emission_rate(1, Duration::Time(10), 1_000);

    // skip 1/10th of the time
    suite.skip_seconds(100_000);

    suite.assert_pending_rewards(ADDR1, 1, 7_500_000);
    suite.assert_pending_rewards(ADDR2, 1, 8_750_000);
    suite.assert_pending_rewards(ADDR3, 1, 8_750_000);

    // skip 2/10ths of the time
    suite.skip_seconds(200_000);

    suite.assert_pending_rewards(ADDR1, 1, 17_500_000);
    suite.assert_pending_rewards(ADDR2, 1, 13_750_000);
    suite.assert_pending_rewards(ADDR3, 1, 13_750_000);

    // pause the rewards distribution
    suite.pause_emission(1);

    // skip 1/10th of the time
    suite.skip_blocks(100_000);

    // assert no pending rewards changed
    suite.assert_pending_rewards(ADDR1, 1, 17_500_000);
    suite.assert_pending_rewards(ADDR2, 1, 13_750_000);
    suite.assert_pending_rewards(ADDR3, 1, 13_750_000);

    // assert ADDR1 pre-claim balance
    suite.assert_native_balance(ADDR1, DENOM, 10_000_000);
    // ADDR1 claims their rewards
    suite.claim_rewards(ADDR1, 1);
    // assert ADDR1 post-claim balance to be pre-claim + pending
    suite.assert_native_balance(ADDR1, DENOM, 10_000_000 + 17_500_000);
    // assert ADDR1 is now entitled to 0 pending rewards
    suite.assert_pending_rewards(ADDR1, 1, 0);

    // user 2 unstakes their stake
    suite.unstake_native_tokens(ADDR2, 50);

    // skip 1/10th of the time
    suite.skip_blocks(100_000);

    // only the ADDR1 pending rewards should have changed
    suite.assert_pending_rewards(ADDR1, 1, 0);
    suite.assert_pending_rewards(ADDR2, 1, 13_750_000);
    suite.assert_pending_rewards(ADDR3, 1, 13_750_000);

    // ADDR2 claims their rewards (has 50 to begin with as they unstaked)
    suite.assert_native_balance(ADDR2, DENOM, 50);
    suite.claim_rewards(ADDR2, 1);
    // assert ADDR2 post-claim balance to be pre-claim + pending and has 0 pending rewards
    suite.assert_native_balance(ADDR2, DENOM, 13_750_000 + 50);
    suite.assert_pending_rewards(ADDR2, 1, 0);

    // update the reward rate back to 1_000 / 10blocks
    // this should now distribute 10_000_000 tokens over 100_000 blocks
    // between ADDR1 (2/3rds) and ADDR3 (1/3rd)
    suite.update_emission_rate(1, Duration::Height(10), 1000);

    // skip 1/10th of the time
    suite.skip_blocks(100_000);

    // assert that rewards are being distributed at the expected rate
    suite.assert_pending_rewards(ADDR1, 1, 6_666_666);
    suite.assert_pending_rewards(ADDR2, 1, 0);
    suite.assert_pending_rewards(ADDR3, 1, 13_750_000 + 3_333_333);

    // ADDR3 claims their rewards
    suite.assert_native_balance(ADDR3, DENOM, 0);
    suite.claim_rewards(ADDR3, 1);
    suite.assert_pending_rewards(ADDR3, 1, 0);
    suite.assert_native_balance(ADDR3, DENOM, 13_750_000 + 3_333_333);

    // skip 1/10th of the time
    suite.skip_blocks(100_000);

    suite.assert_pending_rewards(ADDR1, 1, 6_666_666 + 6_666_666 + 1);
    suite.assert_pending_rewards(ADDR2, 1, 0);
    suite.assert_pending_rewards(ADDR3, 1, 3_333_333);

    // claim everything so that there are 0 pending rewards
    suite.claim_rewards(ADDR3, 1);
    suite.claim_rewards(ADDR1, 1);

    suite.assert_pending_rewards(ADDR1, 1, 0);
    suite.assert_pending_rewards(ADDR2, 1, 0);
    suite.assert_pending_rewards(ADDR3, 1, 0);

    // update the rewards rate to 40_000_000 per 100_000 seconds.
    // split is still 2/3rds to ADDR1 and 1/3rd to ADDR3
    suite.update_emission_rate(1, Duration::Time(10), 4000);
    suite.assert_ends_at(Expiration::AtTime(Timestamp::from_seconds(462_500)));

    suite.skip_seconds(50_000); // allocates 20_000_000 tokens

    let addr1_pending = 20_000_000 * 2 / 3;
    let addr3_pending = 20_000_000 / 3;
    suite.assert_pending_rewards(ADDR1, 1, addr1_pending);
    suite.assert_pending_rewards(ADDR2, 1, 0);
    suite.assert_pending_rewards(ADDR3, 1, addr3_pending);

    // ADDR2 wakes up to the increased staking rate and stakes 50 tokens
    // this brings new split to: [ADDR1: 50%, ADDR2: 25%, ADDR3: 25%]
    suite.stake_native_tokens(ADDR2, 50);

    suite.skip_seconds(10_000); // allocates 4_000_000 tokens

    suite.assert_pending_rewards(ADDR1, 1, addr1_pending + 4_000_000 * 2 / 4);
    suite.assert_pending_rewards(ADDR2, 1, 4_000_000 / 4);
    suite.assert_pending_rewards(ADDR3, 1, addr3_pending + 4_000_000 / 4);

    suite.claim_rewards(ADDR1, 1);
    suite.claim_rewards(ADDR3, 1);
    let addr1_pending = 0;
    let addr3_pending = 0;
    suite.skip_seconds(10_000); // skips from 460,000 to 470,000, and the end is 462,500, so this allocates only 1_000_000 tokens instead of 4_000_000

    suite.assert_pending_rewards(ADDR1, 1, addr1_pending + 1_000_000 * 2 / 4);
    suite.assert_pending_rewards(ADDR2, 1, 4_000_000 / 4 + 1_000_000 / 4);
    suite.assert_pending_rewards(ADDR3, 1, addr3_pending + 1_000_000 / 4);

    suite.claim_rewards(ADDR2, 1);

    // TODO: there's a few denoms remaining here, ensure such cases are handled properly
    let remaining_rewards = suite.get_balance_native(suite.distribution_contract.clone(), DENOM);
    println!("Remaining rewards: {}", remaining_rewards);
}

#[test]
fn test_cw20_dao_native_rewards_block_height_based() {
    let mut suite = SuiteBuilder::base(super::suite::DaoType::CW20).build();

    suite.assert_amount(1_000);
    suite.assert_ends_at(Expiration::AtHeight(1_000_000));
    suite.assert_duration(10);

    // skip 1/10th of the time
    suite.skip_blocks(100_000);

    suite.assert_pending_rewards(ADDR1, 1, 5_000_000);
    suite.assert_pending_rewards(ADDR2, 1, 2_500_000);
    suite.assert_pending_rewards(ADDR3, 1, 2_500_000);

    // skip 1/10th of the time
    suite.skip_blocks(100_000);

    suite.assert_pending_rewards(ADDR1, 1, 10_000_000);
    suite.assert_pending_rewards(ADDR2, 1, 5_000_000);
    suite.assert_pending_rewards(ADDR3, 1, 5_000_000);

    // ADDR1 claims rewards
    suite.claim_rewards(ADDR1, 1);
    suite.assert_native_balance(ADDR1, DENOM, 10_000_000);
    suite.assert_pending_rewards(ADDR1, 1, 0);

    // ADDR2 and ADDR3 unstake their rewards
    suite.unstake_cw20_tokens(50, ADDR2);
    suite.unstake_cw20_tokens(50, ADDR3);

    // skip 1/10th of the time
    suite.skip_blocks(100_000);

    // because ADDR2 and ADDR3 are not staking, ADDR1 receives all the rewards.
    // ADDR2 and ADDR3 should have the same amount of pending rewards as before.
    suite.assert_pending_rewards(ADDR1, 1, 10_000_000);
    suite.assert_pending_rewards(ADDR2, 1, 5_000_000);
    suite.assert_pending_rewards(ADDR3, 1, 5_000_000);

    // ADDR2 and ADDR3 wake up, claim and restake their rewards
    suite.claim_rewards(ADDR2, 1);
    suite.claim_rewards(ADDR3, 1);

    suite.stake_cw20_tokens(50, ADDR2);

    // skip 3/10th of the time
    suite.skip_blocks(300_000);

    suite.stake_cw20_tokens(50, ADDR3);

    suite.assert_pending_rewards(ADDR1, 1, 30_000_000);
    suite.assert_pending_rewards(ADDR2, 1, 10_000_000);
    suite.assert_pending_rewards(ADDR3, 1, 0);

    suite.claim_rewards(ADDR1, 1);
    suite.claim_rewards(ADDR2, 1);

    suite.assert_pending_rewards(ADDR1, 1, 0);
    suite.assert_pending_rewards(ADDR2, 1, 0);
    suite.assert_pending_rewards(ADDR3, 1, 0);

    let remaining_time = suite.get_time_until_rewards_expiration();

    suite.skip_blocks(remaining_time - 100_000);

    suite.claim_rewards(ADDR1, 1);
    suite.unstake_cw20_tokens(100, ADDR1);
    suite.assert_pending_rewards(ADDR1, 1, 0);

    suite.skip_blocks(100_000);

    suite.unstake_cw20_tokens(50, ADDR2);
    suite.skip_blocks(100_000);

    suite.claim_rewards(ADDR2, 1);
    suite.claim_rewards(ADDR3, 1);

    suite.assert_pending_rewards(ADDR1, 1, 0);
    suite.assert_pending_rewards(ADDR2, 1, 0);
    suite.assert_pending_rewards(ADDR3, 1, 0);

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

    suite.assert_pending_rewards(ADDR1, 1, 5_000_000);
    suite.assert_pending_rewards(ADDR2, 1, 2_500_000);
    suite.assert_pending_rewards(ADDR3, 1, 2_500_000);

    // skip 1/10th of the time
    suite.skip_blocks(100_000);

    suite.assert_pending_rewards(ADDR1, 1, 10_000_000);
    suite.assert_pending_rewards(ADDR2, 1, 5_000_000);
    suite.assert_pending_rewards(ADDR3, 1, 5_000_000);

    // ADDR1 claims rewards
    suite.claim_rewards(ADDR1, 1);
    suite.assert_native_balance(ADDR1, DENOM, 10_000_000);
    suite.assert_pending_rewards(ADDR1, 1, 0);

    // ADDR2 and ADDR3 unstake their nfts
    suite.unstake_nft(ADDR2, 3);
    suite.unstake_nft(ADDR3, 4);

    // skip 1/10th of the time
    suite.skip_blocks(100_000);

    // because ADDR2 and ADDR3 are not staking, ADDR1 receives all the rewards.
    // ADDR2 and ADDR3 should have the same amount of pending rewards as before.
    suite.assert_pending_rewards(ADDR1, 1, 10_000_000);
    suite.assert_pending_rewards(ADDR2, 1, 5_000_000);
    suite.assert_pending_rewards(ADDR3, 1, 5_000_000);

    // ADDR2 and ADDR3 wake up, claim and restake their nfts
    suite.claim_rewards(ADDR2, 1);
    suite.claim_rewards(ADDR3, 1);

    suite.stake_nft(ADDR2, 3);
    suite.stake_nft(ADDR3, 4);
}

#[test]
#[should_panic(expected = "No rewards claimable")]
fn test_claim_zero_rewards() {
    let mut suite = SuiteBuilder::base(super::suite::DaoType::CW20).build();

    // skip 1/10th of the time
    suite.skip_blocks(100_000);

    suite.assert_pending_rewards(ADDR1, 1, 5_000_000);

    // ADDR1 claims rewards
    suite.claim_rewards(ADDR1, 1);

    // ADDR1 attempts to claim again
    suite.claim_rewards(ADDR1, 1);
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
            continuous: true,
        })
        .build();

    let cw20_denom = &suite.reward_denom.clone();

    suite.assert_amount(1_000);
    suite.assert_duration(10);
    suite.assert_ends_at(Expiration::AtTime(Timestamp::from_seconds(1_000_000)));

    // skip 1/10th of the time
    suite.skip_seconds(100_000);

    // suite.assert_pending_rewards(ADDR1, 1, 5_000_000);
    suite.assert_pending_rewards(ADDR2, 1, 2_500_000);
    suite.assert_pending_rewards(ADDR3, 1, 2_500_000);

    // skip 1/10th of the time
    suite.skip_seconds(100_000);

    suite.assert_pending_rewards(ADDR1, 1, 10_000_000);
    suite.assert_pending_rewards(ADDR2, 1, 5_000_000);
    suite.assert_pending_rewards(ADDR3, 1, 5_000_000);

    // ADDR1 claims rewards
    suite.claim_rewards(ADDR1, 1);
    suite.assert_cw20_balance(cw20_denom, ADDR1, 10_000_000);
    suite.assert_pending_rewards(ADDR1, 1, 0);

    // ADDR2 and ADDR3 unstake their stake
    suite.unstake_cw20_tokens(50, ADDR2);
    suite.unstake_cw20_tokens(50, ADDR3);

    // skip 1/10th of the time
    suite.skip_seconds(100_000);

    // because ADDR2 and ADDR3 are not staking, ADDR1 receives all the rewards.
    // ADDR2 and ADDR3 should have the same amount of pending rewards as before.
    suite.assert_pending_rewards(ADDR1, 1, 10_000_000);
    suite.assert_pending_rewards(ADDR2, 1, 5_000_000);
    suite.assert_pending_rewards(ADDR3, 1, 5_000_000);

    // ADDR2 and ADDR3 wake up and claim their rewards
    suite.claim_rewards(ADDR2, 1);
    suite.claim_rewards(ADDR3, 1);

    suite.assert_cw20_balance(cw20_denom, ADDR1, 10_000_000);
    suite.assert_cw20_balance(cw20_denom, ADDR2, 5_000_000);
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
            continuous: true,
        })
        .build();

    suite.assert_amount(1_000);
    suite.assert_duration(10);
    suite.assert_ends_at(Expiration::AtTime(Timestamp::from_seconds(1_000_000)));

    // skip 1/10th of the time
    suite.skip_seconds(100_000);

    // suite.assert_pending_rewards(ADDR1, 1, 5_000_000);
    suite.assert_pending_rewards(ADDR2, 1, 2_500_000);
    suite.assert_pending_rewards(ADDR3, 1, 2_500_000);

    // skip 1/10th of the time
    suite.skip_seconds(100_000);

    suite.assert_pending_rewards(ADDR1, 1, 10_000_000);
    suite.assert_pending_rewards(ADDR2, 1, 5_000_000);
    suite.assert_pending_rewards(ADDR3, 1, 5_000_000);

    // ADDR1 claims rewards
    suite.claim_rewards(ADDR1, 1);
    suite.assert_native_balance(ADDR1, DENOM, 10_000_000);
    suite.assert_pending_rewards(ADDR1, 1, 0);

    // ADDR2 and ADDR3 unstake their stake
    suite.unstake_native_tokens(ADDR2, 50);
    suite.unstake_native_tokens(ADDR3, 50);

    // skip 1/10th of the time
    suite.skip_seconds(100_000);

    // because ADDR2 and ADDR3 are not staking, ADDR1 receives all the rewards.
    // ADDR2 and ADDR3 should have the same amount of pending rewards as before.
    suite.assert_pending_rewards(ADDR1, 1, 10_000_000);
    suite.assert_pending_rewards(ADDR2, 1, 5_000_000);
    suite.assert_pending_rewards(ADDR3, 1, 5_000_000);

    // ADDR2 and ADDR3 wake up, claim and restake their rewards
    suite.claim_rewards(ADDR2, 1);
    suite.claim_rewards(ADDR3, 1);

    let addr1_balance = suite.get_balance_native(ADDR1, DENOM);
    let addr2_balance = suite.get_balance_native(ADDR2, DENOM);

    suite.stake_native_tokens(ADDR1, addr1_balance);
    suite.stake_native_tokens(ADDR2, addr2_balance);
}

// all of the `+1` corrections highlight rounding
#[test]
fn test_native_dao_rewards_time_based_with_rounding() {
    // 100udenom/100sec = 1udenom/1sec reward emission rate
    // given funding of 100_000_000udenom, we have a reward duration of 100_000_000sec
    let mut suite = SuiteBuilder::base(super::suite::DaoType::CW4)
        .with_rewards_config(RewardsConfig {
            amount: 100,
            denom: UncheckedDenom::Native(DENOM.to_string()),
            duration: Duration::Time(100),
            destination: None,
            continuous: true,
        })
        .with_cw4_members(vec![
            Member {
                addr: ADDR1.to_string(),
                weight: 140,
            },
            Member {
                addr: ADDR2.to_string(),
                weight: 40,
            },
            Member {
                addr: ADDR3.to_string(),
                weight: 20,
            },
        ])
        .build();

    suite.assert_amount(100);
    suite.assert_duration(100);
    suite.assert_ends_at(Expiration::AtTime(Timestamp::from_seconds(100_000_000)));

    // skip 1 interval
    suite.skip_seconds(100);

    suite.assert_pending_rewards(ADDR1, 1, 70);
    suite.assert_pending_rewards(ADDR2, 1, 20);
    suite.assert_pending_rewards(ADDR3, 1, 10);

    // change voting power of one of the members and claim
    suite.update_members(
        vec![Member {
            addr: ADDR2.to_string(),
            weight: 60,
        }],
        vec![],
    );
    suite.claim_rewards(ADDR2, 1);
    suite.assert_native_balance(ADDR2, DENOM, 20);
    suite.assert_pending_rewards(ADDR2, 1, 0);

    // skip 1 interval
    suite.skip_seconds(100);

    suite.assert_pending_rewards(ADDR1, 1, 70 + 63);
    suite.assert_pending_rewards(ADDR2, 1, 27);
    suite.assert_pending_rewards(ADDR3, 1, 10 + 9);

    // increase reward rate and claim
    suite.update_emission_rate(1, Duration::Time(100), 150);
    suite.claim_rewards(ADDR3, 1);
    suite.assert_native_balance(ADDR3, DENOM, 10 + 9);
    suite.assert_pending_rewards(ADDR3, 1, 0);

    // skip 1 interval
    suite.skip_seconds(100);

    suite.assert_pending_rewards(ADDR1, 1, 70 + 63 + 95 + 1);
    suite.assert_pending_rewards(ADDR2, 1, 27 + 40 + 1);
    suite.assert_pending_rewards(ADDR3, 1, 13);

    // claim rewards
    suite.claim_rewards(ADDR1, 1);
    suite.assert_native_balance(ADDR1, DENOM, 70 + 63 + 95 + 1);
    suite.assert_pending_rewards(ADDR1, 1, 0);

    // skip 3 intervals
    suite.skip_seconds(300);

    suite.assert_pending_rewards(ADDR1, 1, 3 * 95 + 1);
    suite.assert_pending_rewards(ADDR2, 1, 27 + 4 * 40 + 1 + 1 + 1);
    suite.assert_pending_rewards(ADDR3, 1, 4 * 13 + 1 + 1);

    // change voting power for all
    suite.update_members(
        vec![
            Member {
                addr: ADDR1.to_string(),
                weight: 100,
            },
            Member {
                addr: ADDR2.to_string(),
                weight: 80,
            },
            Member {
                addr: ADDR3.to_string(),
                weight: 40,
            },
        ],
        vec![],
    );

    suite.claim_rewards(ADDR2, 1);
    suite.assert_native_balance(ADDR2, DENOM, 20 + 27 + 4 * 40 + 1 + 1 + 1);
    suite.assert_pending_rewards(ADDR2, 1, 0);

    // skip 1 interval
    suite.skip_seconds(100);

    suite.assert_pending_rewards(ADDR1, 1, 3 * 95 + 1 + 68);
    suite.assert_pending_rewards(ADDR2, 1, 54);
    suite.assert_pending_rewards(ADDR3, 1, 4 * 13 + 1 + 1 + 27);

    // claim all
    suite.claim_rewards(ADDR1, 1);
    suite.claim_rewards(ADDR2, 1);
    suite.claim_rewards(ADDR3, 1);
    suite.assert_native_balance(ADDR1, DENOM, 70 + 63 + 95 + 1 + 3 * 95 + 1 + 68);
    suite.assert_native_balance(ADDR2, DENOM, 20 + 27 + 4 * 40 + 1 + 1 + 1 + 54);
    suite.assert_native_balance(ADDR3, DENOM, 10 + 9 + 4 * 13 + 1 + 1 + 27);
    suite.assert_pending_rewards(ADDR1, 1, 0);
    suite.assert_pending_rewards(ADDR2, 1, 0);
    suite.assert_pending_rewards(ADDR3, 1, 0);

    // TODO: fix this rug of 3 udenom by the distribution contract
    suite.assert_native_balance(
        suite.distribution_contract.as_str(),
        DENOM,
        100_000_000 - (100 * 2 + 150 * 5) + 3,
    );
}

#[test]
fn test_native_dao_rewards() {
    let mut suite = SuiteBuilder::base(super::suite::DaoType::Native).build();

    suite.assert_amount(1_000);
    suite.assert_ends_at(Expiration::AtHeight(1_000_000));
    suite.assert_duration(10);

    // skip 1/10th of the time
    suite.skip_blocks(100_000);

    suite.assert_pending_rewards(ADDR1, 1, 5_000_000);
    suite.assert_pending_rewards(ADDR2, 1, 2_500_000);
    suite.assert_pending_rewards(ADDR3, 1, 2_500_000);

    // skip 1/10th of the time
    suite.skip_blocks(100_000);

    suite.assert_pending_rewards(ADDR1, 1, 10_000_000);
    suite.assert_pending_rewards(ADDR2, 1, 5_000_000);
    suite.assert_pending_rewards(ADDR3, 1, 5_000_000);

    // ADDR1 claims rewards
    suite.claim_rewards(ADDR1, 1);
    suite.assert_native_balance(ADDR1, DENOM, 10_000_000);
    suite.assert_pending_rewards(ADDR1, 1, 0);

    // ADDR2 and ADDR3 unstake their stake
    suite.unstake_native_tokens(ADDR2, 50);
    suite.unstake_native_tokens(ADDR3, 50);

    // skip 1/10th of the time
    suite.skip_blocks(100_000);

    // because ADDR2 and ADDR3 are not staking, ADDR1 receives all the rewards.
    // ADDR2 and ADDR3 should have the same amount of pending rewards as before.
    suite.assert_pending_rewards(ADDR1, 1, 10_000_000);
    suite.assert_pending_rewards(ADDR2, 1, 5_000_000);
    suite.assert_pending_rewards(ADDR3, 1, 5_000_000);

    // ADDR2 and ADDR3 wake up, claim and restake their rewards
    suite.claim_rewards(ADDR2, 1);
    suite.claim_rewards(ADDR3, 1);

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

    suite.assert_pending_rewards(ADDR1, 1, 5_000_000);
    suite.assert_pending_rewards(ADDR2, 1, 2_500_000);
    suite.assert_pending_rewards(ADDR3, 1, 2_500_000);

    // remove the second member
    suite.update_members(vec![], vec![ADDR2.to_string()]);
    suite.query_members();

    // skip 1/10th of the time
    suite.skip_blocks(100_000);

    // now that ADDR2 is no longer a member, ADDR1 and ADDR3 will split the rewards
    suite.assert_pending_rewards(ADDR1, 1, 5_000_000 + 6_666_666);
    suite.assert_pending_rewards(ADDR2, 1, 2_500_000);
    suite.assert_pending_rewards(ADDR3, 1, 3_333_333 + 2_500_000);

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
    suite.claim_rewards(ADDR1, 1);
    suite.assert_native_balance(ADDR1, DENOM, 5_000_000 + 6_666_666);

    // assert pending rewards are still the same (other than ADDR1)
    suite.assert_pending_rewards(ADDR1, 1, 0);
    suite.assert_pending_rewards(ADDR2, 1, 2_500_000);
    suite.assert_pending_rewards(ADDR3, 1, 3_333_333 + 2_500_000);

    // skip 1/10th of the time
    suite.skip_blocks(100_000);

    suite.assert_pending_rewards(ADDR1, 1, 4_000_000);
    suite.assert_pending_rewards(ADDR2, 1, 6_500_000);
    suite.assert_pending_rewards(ADDR3, 1, 7_833_333);

    // skip 1/2 of time, leaving 200k blocks left
    suite.skip_blocks(500_000);

    suite.assert_pending_rewards(ADDR1, 1, 24_000_000);
    suite.assert_pending_rewards(ADDR2, 1, 26_500_000);
    suite.assert_pending_rewards(ADDR3, 1, 17_833_333);

    // remove all members
    suite.update_members(
        vec![],
        vec![ADDR1.to_string(), ADDR2.to_string(), ADDR3.to_string()],
    );

    suite.assert_pending_rewards(ADDR1, 1, 24_000_000);
    suite.assert_pending_rewards(ADDR2, 1, 26_500_000);
    suite.assert_pending_rewards(ADDR3, 1, 17_833_333);

    // skip 1/10th of the time
    suite.skip_blocks(100_000);

    suite.assert_pending_rewards(ADDR1, 1, 24_000_000);
    suite.assert_pending_rewards(ADDR2, 1, 26_500_000);
    suite.assert_pending_rewards(ADDR3, 1, 17_833_333);

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

    suite.assert_pending_rewards(ADDR1, 1, 24_000_000);
    suite.assert_pending_rewards(ADDR2, 1, 26_500_000);
    suite.assert_pending_rewards(ADDR3, 1, 17_833_333);

    suite.claim_rewards(ADDR1, 1);
    suite.assert_pending_rewards(ADDR1, 1, 0);
    suite.assert_native_balance(ADDR1, DENOM, 35_666_666);

    // skip 1/10th of the time
    suite.skip_blocks(100_000);

    suite.assert_pending_rewards(ADDR1, 1, 4_000_000);
    suite.assert_pending_rewards(ADDR2, 1, 30_500_000);
    suite.assert_pending_rewards(ADDR3, 1, 19_833_333);

    // at the very expiration block, claim rewards
    suite.claim_rewards(ADDR2, 1);
    suite.assert_pending_rewards(ADDR2, 1, 0);
    suite.assert_native_balance(ADDR2, DENOM, 30_500_000);

    suite.skip_blocks(100_000);

    suite.claim_rewards(ADDR1, 1);
    suite.claim_rewards(ADDR3, 1);

    suite.assert_pending_rewards(ADDR1, 1, 0);
    suite.assert_pending_rewards(ADDR2, 1, 0);
    suite.assert_pending_rewards(ADDR3, 1, 0);

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
    suite.mint_native(alt_coin.clone(), OWNER);
    suite.mint_native(coin.clone(), OWNER);
    let hook_caller = suite.staking_addr.to_string();
    suite.create(
        RewardsConfig {
            amount: 1000,
            denom: cw20::UncheckedDenom::Native(ALT_DENOM.to_string()),
            duration: Duration::Height(100),
            destination: None,
            continuous: true,
        },
        &hook_caller,
        None,
    );

    suite
        .app
        .borrow_mut()
        .execute_contract(
            Addr::unchecked(OWNER),
            suite.distribution_contract.clone(),
            &ExecuteMsg::Fund(FundMsg { id: 2 }),
            &[coin, alt_coin],
        )
        .unwrap();
}

#[test]
#[should_panic(expected = "Invalid CW20")]
fn test_fund_cw20_wrong_denom() {
    let mut suite = SuiteBuilder::base(super::suite::DaoType::Native)
        .with_rewards_config(RewardsConfig {
            amount: 1_000,
            denom: UncheckedDenom::Cw20("irrelevant".to_string()),
            duration: Duration::Height(10),
            destination: None,
            continuous: true,
        })
        .build();

    let mint_cw20 = Cw20Coin {
        address: OWNER.to_string(),
        amount: Uint128::new(100),
    };

    let address = suite.mint_cw20(mint_cw20.clone(), "newcoin").to_string();

    suite.fund_cw20(
        1,
        Cw20Coin {
            address,
            amount: mint_cw20.amount,
        },
    );
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

    let new_cw20_mint = suite.mint_cw20(unregistered_cw20_coin.clone(), "newcoin");

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

    suite.fund_cw20(1, unregistered_cw20_coin);
}

#[test]
#[should_panic(expected = "All rewards have already been distributed")]
fn test_withdraw_finished_rewards_period() {
    let mut suite = SuiteBuilder::base(super::suite::DaoType::Native).build();

    // skip to expiration
    suite.skip_blocks(2_000_000);

    suite.withdraw(1);
}

#[test]
fn test_withdraw_alternative_destination_address() {
    let subdao_addr = "some_subdao_maybe".to_string();
    let mut suite = SuiteBuilder::base(super::suite::DaoType::Native)
        .with_withdraw_destination(Some(subdao_addr.to_string()))
        .build();

    // skip 1/10th of the time
    suite.skip_blocks(100_000);

    suite.assert_pending_rewards(ADDR1, 1, 5_000_000);
    suite.assert_pending_rewards(ADDR2, 1, 2_500_000);
    suite.assert_pending_rewards(ADDR3, 1, 2_500_000);

    // user 1 and 2 claim their rewards
    suite.claim_rewards(ADDR1, 1);
    suite.claim_rewards(ADDR2, 1);

    // user 2 unstakes
    suite.unstake_native_tokens(ADDR2, 50);

    suite.skip_blocks(100_000);

    let distribution_contract = suite.distribution_contract.to_string();

    suite.assert_native_balance(subdao_addr.as_str(), DENOM, 0);
    let pre_withdraw_distributor_balance =
        suite.get_balance_native(distribution_contract.clone(), DENOM);

    suite.withdraw(1);

    let post_withdraw_distributor_balance =
        suite.get_balance_native(distribution_contract.clone(), DENOM);
    let post_withdraw_subdao_balance = suite.get_balance_native(subdao_addr.to_string(), DENOM);

    // after withdraw the balance of the subdao should be the same
    // as pre-withdraw-distributor-bal minus post-withdraw-distributor-bal
    assert_eq!(
        pre_withdraw_distributor_balance - post_withdraw_distributor_balance,
        post_withdraw_subdao_balance
    );
}

#[test]
fn test_withdraw_block_based() {
    let mut suite = SuiteBuilder::base(super::suite::DaoType::Native).build();

    // skip 1/10th of the time
    suite.skip_blocks(100_000);

    // suite.assert_pending_rewards(ADDR1, 1, 5_000_000);
    // suite.assert_pending_rewards(ADDR2, 1, 2_500_000);
    // suite.assert_pending_rewards(ADDR3, 1, 2_500_000);

    // user 1 and 2 claim their rewards
    suite.claim_rewards(ADDR1, 1);
    suite.claim_rewards(ADDR2, 1);

    // user 2 unstakes
    suite.unstake_native_tokens(ADDR2, 50);

    suite.skip_blocks(100_000);

    let distribution_contract = suite.distribution_contract.to_string();

    let pre_withdraw_distributor_balance =
        suite.get_balance_native(distribution_contract.clone(), DENOM);

    suite.assert_native_balance(suite.owner.clone().unwrap().as_str(), DENOM, 0);
    suite.withdraw(1);

    let post_withdraw_distributor_balance =
        suite.get_balance_native(distribution_contract.clone(), DENOM);
    let post_withdraw_owner_balance = suite.get_balance_native(suite.owner.clone().unwrap(), DENOM);

    // after withdraw the balance of the owner should be the same
    // as pre-withdraw-distributor-bal minus post-withdraw-distributor-bal
    assert_eq!(
        pre_withdraw_distributor_balance - post_withdraw_distributor_balance,
        post_withdraw_owner_balance
    );

    assert_eq!(pre_withdraw_distributor_balance, 92_500_000);
    assert_eq!(post_withdraw_distributor_balance, 12_500_000);
    assert_eq!(post_withdraw_owner_balance, 80_000_000);

    suite.skip_blocks(100_000);

    // ensure cannot withdraw again
    assert_eq!(
        suite.withdraw_error(1),
        ContractError::RewardsAlreadyDistributed {}
    );

    // we assert that pending rewards did not change
    suite.assert_pending_rewards(ADDR1, 1, 6_666_666);
    suite.assert_pending_rewards(ADDR2, 1, 0);
    suite.assert_pending_rewards(ADDR3, 1, 3_333_333 + 2_500_000);

    // user 1 can claim their rewards
    suite.claim_rewards(ADDR1, 1);
    // suite.assert_pending_rewards(ADDR1, 1, 0);
    suite.assert_native_balance(ADDR1, DENOM, 11_666_666);

    // user 3 can unstake and claim their rewards
    suite.unstake_native_tokens(ADDR3, 50);
    suite.skip_blocks(100_000);
    suite.assert_native_balance(ADDR3, DENOM, 50);
    suite.claim_rewards(ADDR3, 1);
    // suite.assert_pending_rewards(ADDR3, 1, 0);
    suite.assert_native_balance(ADDR3, DENOM, 3_333_333 + 2_500_000 + 50);

    // TODO: fix this rug of 1 udenom by the distribution contract
    suite.assert_native_balance(&distribution_contract, DENOM, 1);
}

#[test]
fn test_withdraw_time_based() {
    let mut suite = SuiteBuilder::base(super::suite::DaoType::Native)
        .with_rewards_config(RewardsConfig {
            amount: 1_000,
            denom: UncheckedDenom::Native(DENOM.to_string()),
            duration: Duration::Time(10),
            destination: None,
            continuous: true,
        })
        .build();

    // skip 1/10th of the time
    suite.skip_seconds(100_000);

    suite.assert_pending_rewards(ADDR1, 1, 5_000_000);
    suite.assert_pending_rewards(ADDR2, 1, 2_500_000);
    suite.assert_pending_rewards(ADDR3, 1, 2_500_000);

    // user 1 and 2 claim their rewards
    suite.claim_rewards(ADDR1, 1);
    suite.claim_rewards(ADDR2, 1);

    // user 2 unstakes
    suite.unstake_native_tokens(ADDR2, 50);

    suite.skip_seconds(100_000);

    let distribution_contract = suite.distribution_contract.to_string();

    let pre_withdraw_distributor_balance =
        suite.get_balance_native(distribution_contract.clone(), DENOM);

    suite.assert_native_balance(suite.owner.clone().unwrap().as_str(), DENOM, 0);
    suite.withdraw(1);

    let post_withdraw_distributor_balance =
        suite.get_balance_native(distribution_contract.clone(), DENOM);
    let post_withdraw_owner_balance = suite.get_balance_native(suite.owner.clone().unwrap(), DENOM);

    // after withdraw the balance of the owner should be the same
    // as pre-withdraw-distributor-bal minus post-withdraw-distributor-bal
    assert_eq!(
        pre_withdraw_distributor_balance - post_withdraw_distributor_balance,
        post_withdraw_owner_balance
    );

    assert_eq!(pre_withdraw_distributor_balance, 92_500_000);
    assert_eq!(post_withdraw_distributor_balance, 12_500_000);
    assert_eq!(post_withdraw_owner_balance, 80_000_000);

    suite.skip_seconds(100_000);

    // ensure cannot withdraw again
    assert_eq!(
        suite.withdraw_error(1),
        ContractError::RewardsAlreadyDistributed {}
    );

    // we assert that pending rewards did not change
    suite.assert_pending_rewards(ADDR1, 1, 6_666_666);
    suite.assert_pending_rewards(ADDR2, 1, 0);
    suite.assert_pending_rewards(ADDR3, 1, 3_333_333 + 2_500_000);

    // user 1 can claim their rewards
    suite.claim_rewards(ADDR1, 1);
    suite.assert_pending_rewards(ADDR1, 1, 0);
    suite.assert_native_balance(ADDR1, DENOM, 11_666_666);

    // user 3 can unstake and claim their rewards
    suite.unstake_native_tokens(ADDR3, 50);
    suite.skip_seconds(100_000);
    suite.assert_native_balance(ADDR3, DENOM, 50);
    suite.claim_rewards(ADDR3, 1);
    suite.assert_pending_rewards(ADDR3, 1, 0);
    suite.assert_native_balance(ADDR3, DENOM, 3_333_333 + 2_500_000 + 50);

    // TODO: fix this rug of 1 udenom by the distribution contract
    suite.assert_native_balance(&distribution_contract, DENOM, 1);
}

#[test]
fn test_withdraw_and_restart_with_continuous() {
    let mut suite = SuiteBuilder::base(super::suite::DaoType::Native)
        .with_rewards_config(RewardsConfig {
            amount: 1_000,
            denom: UncheckedDenom::Native(DENOM.to_string()),
            duration: Duration::Time(10),
            destination: None,
            continuous: true,
        })
        .build();

    // skip 1/10th of the time
    suite.skip_seconds(100_000);

    suite.assert_pending_rewards(ADDR1, 1, 5_000_000);
    suite.assert_pending_rewards(ADDR2, 1, 2_500_000);
    suite.assert_pending_rewards(ADDR3, 1, 2_500_000);

    // users claim their rewards
    suite.claim_rewards(ADDR1, 1);
    suite.claim_rewards(ADDR2, 1);
    suite.claim_rewards(ADDR3, 1);

    // skip 1/10th of the time
    suite.skip_seconds(100_000);

    let distribution_contract = suite.distribution_contract.to_string();

    let pre_withdraw_distributor_balance =
        suite.get_balance_native(distribution_contract.clone(), DENOM);

    suite.assert_native_balance(suite.owner.clone().unwrap().as_str(), DENOM, 0);
    suite.withdraw(1);

    let post_withdraw_distributor_balance =
        suite.get_balance_native(distribution_contract.clone(), DENOM);
    let post_withdraw_owner_balance = suite.get_balance_native(suite.owner.clone().unwrap(), DENOM);

    // after withdraw the balance of the owner should be the same
    // as pre-withdraw-distributor-bal minus post-withdraw-distributor-bal
    assert_eq!(
        pre_withdraw_distributor_balance - post_withdraw_distributor_balance,
        post_withdraw_owner_balance
    );

    assert_eq!(pre_withdraw_distributor_balance, 90_000_000);
    assert_eq!(post_withdraw_distributor_balance, 10_000_000);
    assert_eq!(post_withdraw_owner_balance, 80_000_000);

    // skip 1/10th of the time
    suite.skip_seconds(100_000);

    // ensure cannot withdraw again
    assert_eq!(
        suite.withdraw_error(1),
        ContractError::RewardsAlreadyDistributed {}
    );

    // we assert that pending rewards did not change
    suite.assert_pending_rewards(ADDR1, 1, 5_000_000);
    suite.assert_pending_rewards(ADDR2, 1, 2_500_000);
    suite.assert_pending_rewards(ADDR3, 1, 2_500_000);
    suite.claim_rewards(ADDR1, 1);
    suite.claim_rewards(ADDR2, 1);
    suite.claim_rewards(ADDR3, 1);

    // fund again
    suite.fund_native(1, coin(100_000_000, DENOM));

    // check that pending rewards did not restart. since we skipped 1/10th the
    // time after the withdraw occurred, everyone should already have 10% of the
    // new amount pending.
    suite.assert_pending_rewards(ADDR1, 1, 5_000_000);
    suite.assert_pending_rewards(ADDR2, 1, 2_500_000);
    suite.assert_pending_rewards(ADDR3, 1, 2_500_000);
}

#[test]
fn test_withdraw_and_restart_not_continuous() {
    let mut suite = SuiteBuilder::base(super::suite::DaoType::Native)
        .with_rewards_config(RewardsConfig {
            amount: 1_000,
            denom: UncheckedDenom::Native(DENOM.to_string()),
            duration: Duration::Time(10),
            destination: None,
            continuous: false,
        })
        .build();

    // skip 1/10th of the time
    suite.skip_seconds(100_000);

    suite.assert_pending_rewards(ADDR1, 1, 5_000_000);
    suite.assert_pending_rewards(ADDR2, 1, 2_500_000);
    suite.assert_pending_rewards(ADDR3, 1, 2_500_000);

    // users claim their rewards
    suite.claim_rewards(ADDR1, 1);
    suite.claim_rewards(ADDR2, 1);
    suite.claim_rewards(ADDR3, 1);

    // skip 1/10th of the time
    suite.skip_seconds(100_000);

    let distribution_contract = suite.distribution_contract.to_string();

    let pre_withdraw_distributor_balance =
        suite.get_balance_native(distribution_contract.clone(), DENOM);

    suite.assert_native_balance(suite.owner.clone().unwrap().as_str(), DENOM, 0);
    suite.withdraw(1);

    let post_withdraw_distributor_balance =
        suite.get_balance_native(distribution_contract.clone(), DENOM);
    let post_withdraw_owner_balance = suite.get_balance_native(suite.owner.clone().unwrap(), DENOM);

    // after withdraw the balance of the owner should be the same
    // as pre-withdraw-distributor-bal minus post-withdraw-distributor-bal
    assert_eq!(
        pre_withdraw_distributor_balance - post_withdraw_distributor_balance,
        post_withdraw_owner_balance
    );

    assert_eq!(pre_withdraw_distributor_balance, 90_000_000);
    assert_eq!(post_withdraw_distributor_balance, 10_000_000);
    assert_eq!(post_withdraw_owner_balance, 80_000_000);

    // skip 1/10th of the time
    suite.skip_seconds(100_000);

    // ensure cannot withdraw again
    assert_eq!(
        suite.withdraw_error(1),
        ContractError::RewardsAlreadyDistributed {}
    );

    // we assert that pending rewards did not change
    suite.assert_pending_rewards(ADDR1, 1, 5_000_000);
    suite.assert_pending_rewards(ADDR2, 1, 2_500_000);
    suite.assert_pending_rewards(ADDR3, 1, 2_500_000);
    suite.claim_rewards(ADDR1, 1);
    suite.claim_rewards(ADDR2, 1);
    suite.claim_rewards(ADDR3, 1);

    // fund again
    suite.fund_native(1, coin(100_000_000, DENOM));

    // skip 1/10th of the time
    suite.skip_seconds(100_000);

    // check that pending rewards restarted from the funding date. since we
    // skipped 1/10th the time after the funding occurred, everyone should
    // have 10% of the new amount pending
    suite.assert_pending_rewards(ADDR1, 1, 5_000_000);
    suite.assert_pending_rewards(ADDR2, 1, 2_500_000);
    suite.assert_pending_rewards(ADDR3, 1, 2_500_000);
}

#[test]
#[should_panic(expected = "Caller is not the contract's current owner")]
fn test_withdraw_unauthorized() {
    let mut suite = SuiteBuilder::base(super::suite::DaoType::Native).build();

    // skip 1/10th of the time
    suite.skip_blocks(100_000);

    suite
        .app
        .borrow_mut()
        .execute_contract(
            Addr::unchecked(ADDR1),
            suite.distribution_contract.clone(),
            &ExecuteMsg::Withdraw { id: 1 },
            &[],
        )
        .unwrap();
}

#[test]
#[should_panic(expected = "Distribution not found with ID 3")]
fn test_withdraw_404() {
    let mut suite = SuiteBuilder::base(super::suite::DaoType::Native).build();

    suite.skip_blocks(100_000);

    suite.withdraw(3);
}

#[test]
#[should_panic(expected = "Distribution not found with ID 3")]
fn test_claim_404() {
    let mut suite = SuiteBuilder::base(super::suite::DaoType::Native).build();

    suite.skip_blocks(100_000);

    suite.claim_rewards(ADDR1, 3);
}

#[test]
#[should_panic]
fn test_fund_invalid_native_denom() {
    let mut suite = SuiteBuilder::base(super::suite::DaoType::Native).build();

    // skip 1/10th of the time
    suite.skip_blocks(100_000);

    suite.mint_native(coin(100_000_000, ALT_DENOM), OWNER);
    suite
        .app
        .borrow_mut()
        .execute_contract(
            Addr::unchecked(OWNER),
            suite.distribution_contract.clone(),
            &ExecuteMsg::Fund(FundMsg { id: 1 }),
            &[coin(100_000_000, ALT_DENOM)],
        )
        .unwrap();
}

#[test]
fn test_fund_native_block_based_post_expiration_not_continuous() {
    let mut suite = SuiteBuilder::base(super::suite::DaoType::Native)
        .with_rewards_config(RewardsConfig {
            amount: 1_000,
            denom: UncheckedDenom::Native(DENOM.to_string()),
            duration: Duration::Height(10),
            destination: None,
            continuous: false,
        })
        .build();

    let started_at = Expiration::AtHeight(0);
    let funded_blocks = 1_000_000;
    let expiration_date = Expiration::AtHeight(funded_blocks);
    suite.assert_amount(1_000);
    suite.assert_ends_at(expiration_date);
    suite.assert_started_at(started_at);
    suite.assert_duration(10);

    // skip 1/10th of the time
    suite.skip_blocks(100_000);

    suite.assert_pending_rewards(ADDR1, 1, 5_000_000);
    suite.assert_pending_rewards(ADDR2, 1, 2_500_000);
    suite.assert_pending_rewards(ADDR3, 1, 2_500_000);

    // ADDR2 unstake their stake
    suite.unstake_native_tokens(ADDR2, 50);

    // addr3 claims their rewards
    suite.claim_rewards(ADDR3, 1);

    // skip to 100_000 blocks past the expiration
    suite.skip_blocks(1_000_000);

    suite.assert_pending_rewards(ADDR1, 1, 65_000_000);
    suite.assert_pending_rewards(ADDR2, 1, 2_500_000);
    suite.assert_pending_rewards(ADDR3, 1, 30_000_000);

    suite.assert_ends_at(expiration_date);
    suite.assert_started_at(started_at);

    // we fund the distributor with the same amount of coins as
    // during setup, meaning that the rewards distribution duration
    // should be the same.
    suite.fund_native(1, coin(100_000_000, DENOM));

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
fn test_fund_cw20_time_based_post_expiration_not_continuous() {
    let mut suite = SuiteBuilder::base(super::suite::DaoType::CW20)
        .with_rewards_config(RewardsConfig {
            amount: 1_000,
            denom: UncheckedDenom::Cw20(DENOM.to_string()),
            duration: Duration::Time(10),
            destination: None,
            continuous: false,
        })
        .build();

    let cw20_denom = &suite.reward_denom.clone();

    let started_at = Expiration::AtTime(Timestamp::from_seconds(0));
    let funded_timestamp = Timestamp::from_seconds(1_000_000);
    let expiration_date = Expiration::AtTime(funded_timestamp);
    suite.assert_amount(1_000);
    suite.assert_ends_at(expiration_date);
    suite.assert_started_at(started_at);
    suite.assert_duration(10);

    // skip 1/10th of the time
    suite.skip_seconds(100_000);

    suite.assert_pending_rewards(ADDR1, 1, 5_000_000);
    suite.assert_pending_rewards(ADDR2, 1, 2_500_000);
    suite.assert_pending_rewards(ADDR3, 1, 2_500_000);

    // ADDR2 unstake their stake
    suite.unstake_cw20_tokens(50, ADDR2);

    // addr3 claims their rewards
    suite.claim_rewards(ADDR3, 1);
    suite.assert_cw20_balance(cw20_denom, ADDR3, 2_500_000);

    // skip to 100_000 blocks past the expiration
    suite.skip_seconds(1_000_000);

    suite.assert_pending_rewards(ADDR1, 1, 65_000_000);
    suite.assert_pending_rewards(ADDR2, 1, 2_500_000);
    suite.assert_pending_rewards(ADDR3, 1, 30_000_000);

    suite.assert_ends_at(expiration_date);
    suite.assert_started_at(started_at);

    // we fund the distributor with the same amount of coins as
    // during setup, meaning that the rewards distribution duration
    // should be the same.
    let funding_denom = Cw20Coin {
        address: suite.reward_denom.to_string(),
        amount: Uint128::new(100_000_000),
    };

    suite.fund_cw20(1, funding_denom.clone());

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
            continuous: true,
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

    suite.assert_pending_rewards(ADDR1, 1, 5_000_000);
    suite.assert_pending_rewards(ADDR2, 1, 2_500_000);
    suite.assert_pending_rewards(ADDR3, 1, 2_500_000);

    // ADDR2 unstake their stake
    suite.unstake_cw20_tokens(50, ADDR2);

    // addr3 claims their rewards
    suite.claim_rewards(ADDR3, 1);

    // skip to 100_000 blocks before the expiration
    suite.skip_seconds(800_000);

    suite.assert_pending_rewards(ADDR1, 1, 58_333_333);
    suite.assert_pending_rewards(ADDR2, 1, 2_500_000);
    suite.assert_pending_rewards(ADDR3, 1, 26_666_666);

    suite.assert_ends_at(expiration_date);
    suite.assert_started_at(started_at);

    // we fund the distributor with the same amount of coins as
    // during setup, meaning that the rewards distribution duration
    // should be the same.
    let funding_denom = Cw20Coin {
        address: suite.reward_denom.to_string(),
        amount: Uint128::new(100_000_000),
    };
    suite.fund_cw20(1, funding_denom.clone());

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

    suite.assert_pending_rewards(ADDR1, 1, 5_000_000);
    suite.assert_pending_rewards(ADDR2, 1, 2_500_000);
    suite.assert_pending_rewards(ADDR3, 1, 2_500_000);

    // ADDR2 unstake their stake
    suite.unstake_native_tokens(ADDR2, 50);

    // addr3 claims their rewards
    suite.claim_rewards(ADDR3, 1);

    // skip to 100_000 blocks before the expiration
    suite.skip_blocks(800_000);

    suite.assert_pending_rewards(ADDR1, 1, 58_333_333);
    suite.assert_pending_rewards(ADDR2, 1, 2_500_000);
    suite.assert_pending_rewards(ADDR3, 1, 26_666_666);

    suite.assert_ends_at(expiration_date);
    suite.assert_started_at(started_at);

    // we fund the distributor with the same amount of coins as
    // during setup, meaning that the rewards distribution duration
    // should be the same.
    suite.fund_native(1, coin(100_000_000, DENOM));

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
    suite.mint_native(coin(100, DENOM), ADDR1);
    suite.stake_native_tokens(ADDR1, 100);

    // rewards here should not be affected by the new stake,
    suite.assert_pending_rewards(ADDR1, 1, 5_000_000);
    suite.assert_pending_rewards(ADDR2, 1, 2_500_000);
    suite.assert_pending_rewards(ADDR3, 1, 2_500_000);

    // skip 1/10th of the time
    suite.skip_blocks(100_000);

    // here we should see the new stake affecting the rewards split.
    suite.assert_pending_rewards(ADDR1, 1, 5_000_000 + 6_666_666);
    suite.assert_pending_rewards(ADDR2, 1, 2_500_000 + 1_666_666);
    suite.assert_pending_rewards(ADDR3, 1, 2_500_000 + 1_666_666);

    // ADDR1 claims rewards
    suite.claim_rewards(ADDR1, 1);
    suite.assert_native_balance(ADDR1, DENOM, 5_000_000 + 6_666_666);
    suite.assert_pending_rewards(ADDR1, 1, 0);

    // ADDR2 and ADDR3 unstake their stake
    // new voting power split is [ADDR1: 100%, ADDR2: 0%, ADDR3: 0%]
    suite.unstake_native_tokens(ADDR2, 50);
    suite.unstake_native_tokens(ADDR3, 50);

    // we assert that by unstaking, ADDR2 and ADDR3 do not forfeit their earned but unclaimed rewards
    suite.assert_pending_rewards(ADDR2, 1, 2_500_000 + 1_666_666);
    suite.assert_pending_rewards(ADDR3, 1, 2_500_000 + 1_666_666);

    // skip a block and assert that nothing changes
    suite.skip_blocks(1);
    suite.assert_pending_rewards(ADDR2, 1, 2_500_000 + 1_666_666);
    suite.assert_pending_rewards(ADDR3, 1, 2_500_000 + 1_666_666);

    // skip the remaining blocks to reach 1/10th of the time
    suite.skip_blocks(99_999);

    // because ADDR2 and ADDR3 are not staking, ADDR1 receives all the rewards.
    // ADDR2 and ADDR3 should have the same amount of pending rewards as before.
    suite.assert_pending_rewards(ADDR1, 1, 10_000_000);
    suite.assert_pending_rewards(ADDR2, 1, 2_500_000 + 1_666_666);
    suite.assert_pending_rewards(ADDR3, 1, 2_500_000 + 1_666_666);

    // ADDR2 and ADDR3 wake up, claim and restake their rewards
    suite.claim_rewards(ADDR2, 1);
    suite.claim_rewards(ADDR3, 1);

    let addr1_balance = suite.get_balance_native(ADDR1, DENOM);
    let addr2_balance = suite.get_balance_native(ADDR2, DENOM);

    suite.stake_native_tokens(ADDR1, addr1_balance);
    suite.stake_native_tokens(ADDR2, addr2_balance);
}

#[test]
fn test_fund_native_on_create() {
    let mut suite = SuiteBuilder::base(super::suite::DaoType::Native).build();

    let alt_coin = coin(100_000_000, ALT_DENOM);
    suite.mint_native(alt_coin.clone(), OWNER);
    let hook_caller = suite.staking_addr.to_string();

    suite.create(
        RewardsConfig {
            amount: 1000,
            denom: cw20::UncheckedDenom::Native(ALT_DENOM.to_string()),
            duration: Duration::Height(100),
            destination: None,
            continuous: true,
        },
        &hook_caller,
        Some(alt_coin.amount),
    );

    let distribution = suite.get_distribution(2);
    assert_eq!(distribution.funded_amount, alt_coin.amount);
    assert_eq!(
        distribution.active_epoch,
        Epoch {
            emission_rate: EmissionRate::Linear {
                amount: Uint128::new(1000),
                duration: Duration::Height(100),
            },
            started_at: Expiration::AtHeight(0),
            ends_at: Expiration::AtHeight(10_000_000),
            total_earned_puvp: Uint256::zero(),
            last_updated_total_earned_puvp: Expiration::AtHeight(0),
        }
    );

    suite.skip_blocks(1_000_000); // skip 1/10th of the time

    suite.assert_pending_rewards(ADDR1, 2, 5_000_000);
    suite.assert_pending_rewards(ADDR2, 2, 2_500_000);
    suite.assert_pending_rewards(ADDR3, 2, 2_500_000);
}

#[test]
#[should_panic(expected = "Must send reserve token 'ujuno'")]
fn test_fund_native_with_other_denom() {
    let mut suite = SuiteBuilder::base(super::suite::DaoType::Native).build();

    suite.mint_native(coin(100, ALT_DENOM), OWNER);

    let execute_create_msg = ExecuteMsg::Create(CreateMsg {
        denom: cw20::UncheckedDenom::Native(DENOM.to_string()),
        emission_rate: EmissionRate::Linear {
            amount: Uint128::new(1000),
            duration: Duration::Height(100),
        },
        continuous: true,
        hook_caller: suite.staking_addr.to_string(),
        vp_contract: suite.voting_power_addr.to_string(),
        withdraw_destination: None,
    });

    // create distribution with other denom provided
    suite
        .app
        .execute_contract(
            Addr::unchecked(OWNER),
            suite.distribution_contract.clone(),
            &execute_create_msg,
            &coins(100, ALT_DENOM),
        )
        .unwrap();
}

#[test]
#[should_panic(expected = "Sent more than one denomination")]
fn test_fund_native_multiple_denoms() {
    let mut suite = SuiteBuilder::base(super::suite::DaoType::Native).build();

    suite.mint_native(coin(100, DENOM), OWNER);
    suite.mint_native(coin(100, ALT_DENOM), OWNER);

    let execute_create_msg = ExecuteMsg::Create(CreateMsg {
        denom: cw20::UncheckedDenom::Native(DENOM.to_string()),
        emission_rate: EmissionRate::Linear {
            amount: Uint128::new(1000),
            duration: Duration::Height(100),
        },
        continuous: true,
        hook_caller: suite.staking_addr.to_string(),
        vp_contract: suite.voting_power_addr.to_string(),
        withdraw_destination: None,
    });

    // create distribution with 0 amount
    suite
        .app
        .execute_contract(
            Addr::unchecked(OWNER),
            suite.distribution_contract.clone(),
            &execute_create_msg,
            &[coin(100, DENOM), coin(100, ALT_DENOM)],
        )
        .unwrap();
}

#[test]
#[should_panic(expected = "You cannot send native funds when creating a CW20 distribution")]
fn test_fund_native_on_create_cw20() {
    let mut suite = SuiteBuilder::base(super::suite::DaoType::Native).build();

    suite.mint_native(coin(100, DENOM), OWNER);

    let cw20_denom = suite
        .mint_cw20(
            Cw20Coin {
                address: OWNER.to_string(),
                amount: Uint128::new(100),
            },
            "newcoin",
        )
        .to_string();

    let execute_create_msg = ExecuteMsg::Create(CreateMsg {
        denom: cw20::UncheckedDenom::Cw20(cw20_denom),
        emission_rate: EmissionRate::Linear {
            amount: Uint128::new(1000),
            duration: Duration::Height(100),
        },
        continuous: true,
        hook_caller: suite.staking_addr.to_string(),
        vp_contract: suite.voting_power_addr.to_string(),
        withdraw_destination: None,
    });

    // create cw20 distribution with native funds provided
    suite
        .app
        .execute_contract(
            Addr::unchecked(OWNER),
            suite.distribution_contract.clone(),
            &execute_create_msg,
            &coins(100, DENOM),
        )
        .unwrap();
}

#[test]
fn test_update_continuous() {
    let mut suite = SuiteBuilder::base(super::suite::DaoType::Native).build();

    suite.update_continuous(1, true);

    let distribution = suite.get_distribution(1);
    assert!(distribution.continuous);

    suite.update_continuous(1, false);

    let distribution = suite.get_distribution(1);
    assert!(!distribution.continuous);
}

#[test]
fn test_update_owner() {
    let mut suite = SuiteBuilder::base(super::suite::DaoType::Native).build();

    let new_owner = "new_owner";
    suite.update_owner(new_owner);

    let owner = suite.get_owner().to_string();
    assert_eq!(owner, new_owner);
}

#[test]
fn test_update_vp_contract() {
    let mut suite = SuiteBuilder::base(super::suite::DaoType::Native).build();

    let new_vp_contract = setup_native_token_test(suite.app.borrow_mut());

    suite.update_vp_contract(1, new_vp_contract.as_str());

    let distribution = suite.get_distribution(1);
    assert_eq!(distribution.vp_contract, new_vp_contract);
}

#[test]
fn test_update_hook_caller() {
    let mut suite = SuiteBuilder::base(super::suite::DaoType::Native).build();

    let new_hook_caller = "new_hook_caller";
    suite.update_hook_caller(1, new_hook_caller);

    let distribution = suite.get_distribution(1);
    assert_eq!(distribution.hook_caller, new_hook_caller);
}

#[test]
fn test_update_withdraw_destination() {
    let mut suite = SuiteBuilder::base(super::suite::DaoType::Native).build();

    let new_withdraw_destination = "new_withdraw_destination";
    suite.update_withdraw_destination(1, new_withdraw_destination);

    let distribution = suite.get_distribution(1);
    assert_eq!(distribution.withdraw_destination, new_withdraw_destination);
}

#[test]
#[should_panic(expected = "Distribution not found with ID 3")]
fn test_update_404() {
    let mut suite = SuiteBuilder::base(super::suite::DaoType::Native).build();

    suite.update_continuous(3, false);
}

#[test]
#[should_panic(expected = "Invalid emission rate: amount cannot be zero")]
fn test_validate_emission_rate_amount() {
    let mut suite = SuiteBuilder::base(super::suite::DaoType::Native).build();
    suite.update_emission_rate(1, Duration::Time(100), 0);
}

#[test]
#[should_panic(expected = "Invalid emission rate: duration cannot be zero")]
fn test_validate_emission_rate_duration_height() {
    let mut suite = SuiteBuilder::base(super::suite::DaoType::Native).build();
    suite.update_emission_rate(1, Duration::Height(0), 100);
}

#[test]
#[should_panic(expected = "Invalid emission rate: duration cannot be zero")]
fn test_validate_emission_rate_duration_time() {
    let mut suite = SuiteBuilder::base(super::suite::DaoType::Native).build();
    suite.update_emission_rate(1, Duration::Time(0), 100);
}

#[test]
fn test_query_info() {
    let mut suite = SuiteBuilder::base(super::suite::DaoType::Native).build();

    let info = suite.get_info();

    assert_eq!(
        info,
        InfoResponse {
            info: ContractVersion {
                contract: env!("CARGO_PKG_NAME").to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            }
        }
    );
}
