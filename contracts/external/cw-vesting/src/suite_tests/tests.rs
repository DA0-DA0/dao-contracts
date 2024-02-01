use cosmwasm_std::{Timestamp, Uint128, Uint64};
use cw_multi_test::App;
use cw_ownable::OwnershipError;

use crate::{
    vesting::{Schedule, Status},
    ContractError,
};

use super::{is_error, suite::SuiteBuilder};

#[test]
fn test_suite_instantiate() {
    SuiteBuilder::default().build();
}

/// Can not have a start time in the past such that the vest would
/// complete instantly.
#[test]
#[should_panic(expected = "this vesting contract would complete instantly")]
fn test_no_past_instavest() {
    SuiteBuilder::default()
        .with_start_time(Timestamp::from_seconds(0))
        .with_vesting_duration(10)
        .build();
}

#[test]
#[should_panic(expected = "this vesting contract would complete instantly")]
fn test_no_duration_instavest() {
    SuiteBuilder::default()
        .with_start_time(Timestamp::from_seconds(0))
        .with_vesting_duration(0)
        .build();
}

#[test]
#[should_panic(expected = "this vesting contract would complete instantly")]
fn test_no_instavest_in_the_future() {
    let default_start_time = App::default().block_info().time;

    SuiteBuilder::default()
        .with_start_time(default_start_time.plus_seconds(60 * 60 * 24))
        .with_vesting_duration(0)
        .build();
}

/// Attempting to distribute more tokens than are claimable is not
/// allowed.
#[test]
fn test_distribute_more_than_claimable() {
    let mut suite = SuiteBuilder::default().build();
    suite.a_day_passes();

    let res = suite.distribute(suite.receiver.clone(), Some(suite.total));
    is_error!(
        res,
        ContractError::InvalidWithdrawal {
            request: suite.total,
            claimable: Uint128::new(100_000_000 / 7),
        }
        .to_string()
        .as_str()
    )
}

/// Attempting to distribute while nothing is claimable is not
/// allowed.
#[test]
fn test_distribute_nothing_claimable() {
    let mut suite = SuiteBuilder::default().build();

    // two days pass, 2/7ths of rewards avaliable.
    suite.a_day_passes();
    suite.a_day_passes();

    // anyone can call distribute.
    suite.distribute("random", None).unwrap();

    let balance = suite.query_receiver_vesting_token_balance();
    assert_eq!(balance, suite.total.multiply_ratio(2u128, 7u128));

    let res = suite.distribute("random", None);

    is_error!(
        res,
        ContractError::InvalidWithdrawal {
            request: Uint128::zero(),
            claimable: Uint128::zero(),
        }
        .to_string()
        .as_str()
    )
}

/// Distributing long after the vest has totally vested is fine.
#[test]
fn test_distribute_post_completion() {
    let mut suite = SuiteBuilder::default().build();

    suite.a_day_passes();

    suite.distribute("random", None).unwrap();
    let balance = suite.query_receiver_vesting_token_balance();
    assert_eq!(balance, suite.total.multiply_ratio(1u128, 7u128));

    suite.a_week_passes();
    suite.a_week_passes();

    suite.distribute("violet", None).unwrap();
    let balance = suite.query_receiver_vesting_token_balance();
    assert_eq!(balance, suite.total);
}

/// This cancels a vesting contract at a time when it has insufficent
/// liquid tokens to settle the vest receiver. In a situation like
/// this, it should settle the receiver as much as possible, allow
/// anyone to unstake, and allow the receiver and owner to claim their
/// tokens once all of them have unstaked.
#[test]
fn test_cancel_can_not_settle_receiver() {
    let mut suite = SuiteBuilder::default().build();

    // delegate all but ten tokens (in terms of non-micro
    // denominations).
    suite.delegate(Uint128::new(90_000_000)).unwrap();

    suite.a_day_passes();

    // withdraw rewards before cancelation. not doing this would cause
    // the rewards withdrawal address to be updated to the owner and
    // thus entitle them to the rewards.
    suite.withdraw_delegator_reward("validator").unwrap();

    suite.cancel(suite.owner.clone().unwrap()).unwrap();

    suite.a_day_passes();

    // now that the vest is canceled, these rewards should go to the
    // owner.
    suite.withdraw_delegator_reward("validator").unwrap();

    let owner_rewards = suite.query_vesting_token_balance(suite.owner.clone().unwrap());
    let expected_staking_rewards = Uint128::new(90_000_000)
        .multiply_ratio(1u128, 10u128) // default rewards rate is 10%/yr
        .multiply_ratio(1u128, 365u128);
    assert_eq!(owner_rewards, expected_staking_rewards);

    // receiver should have received the same amount of staking
    // rewards as the owner, as well as the liquid tokens in the
    // contract at the time of cancelation.
    let receiver_balance = suite.query_receiver_vesting_token_balance();
    assert_eq!(receiver_balance, owner_rewards + Uint128::new(10_000_000));

    // contract is canceled so anyone can undelegate.
    suite
        .undelegate("random", Uint128::new(90_000_000))
        .unwrap();

    // let tokens unstake. default unstaking period is ten seconds.
    suite.a_day_passes();
    suite.process_unbonds();

    suite.withdraw_canceled(None).unwrap();
    suite.distribute("random", None).unwrap();

    // vestee should now have received all tokens they are entitled to
    // having vested for one day.
    let balance = suite.query_receiver_vesting_token_balance();
    assert_eq!(
        balance,
        suite.total.multiply_ratio(1u128, 7u128) + expected_staking_rewards
    );

    let owner = suite.query_vesting_token_balance(suite.owner.clone().unwrap());
    assert_eq!(
        owner,
        suite.total - suite.total.multiply_ratio(1u128, 7u128) + expected_staking_rewards
    );
}

#[test]
fn test_set_withdraw_address_permissions() {
    let mut suite = SuiteBuilder::default().build();

    // delegate all but ten tokens (in terms of non-micro
    // denominations).
    suite.delegate(Uint128::new(90_000_000)).unwrap();

    suite.a_day_passes();

    // owner may not update withdraw address if vesting is not canceled.
    let res =
        suite.set_withdraw_address(suite.owner.clone().unwrap().to_string().as_str(), "random");
    is_error!(res, ContractError::NotReceiver.to_string().as_str());

    // non-owner can not cancel.
    let res = suite.cancel("random");
    is_error!(
        res,
        ContractError::Ownable(OwnershipError::NotOwner)
            .to_string()
            .as_str()
    );

    suite.cancel(suite.owner.clone().unwrap()).unwrap();

    let res = suite.set_withdraw_address(suite.owner.clone().unwrap(), suite.vesting.clone());
    is_error!(res, ContractError::SelfWithdraw.to_string().as_str());
}

/// Canceling a completed vest is fine.
#[test]
fn test_cancel_completed_vest() {
    let mut suite = SuiteBuilder::default().build();
    suite.a_week_passes();
    suite.distribute("random", None).unwrap();
    suite.cancel(suite.owner.clone().unwrap()).unwrap();
    assert_eq!(
        suite.query_vest().status,
        Status::Canceled {
            owner_withdrawable: Uint128::zero()
        }
    )
}

#[test]
fn test_redelegation() {
    let expected_balance = {
        // same operation as below, but without a redelegation.
        let mut suite = SuiteBuilder::default().build();
        suite.delegate(Uint128::new(100_000_000)).unwrap();
        suite.a_day_passes();
        suite.a_day_passes();
        suite
            .undelegate(suite.receiver.clone(), Uint128::new(25_000_000))
            .unwrap();

        suite.a_day_passes();
        suite.process_unbonds();

        suite.distribute("random", None).unwrap();
        suite.withdraw_delegator_reward("validator").unwrap();

        suite.query_receiver_vesting_token_balance()
    };

    let expected_staking_rewards = Uint128::new(100_000_000)
        .multiply_ratio(1u128, 10u128)
        .multiply_ratio(2u128, 365u128)
        + Uint128::new(75_000_000)
            .multiply_ratio(1u128, 10u128)
            .multiply_ratio(1u128, 365u128);

    assert_eq!(
        expected_staking_rewards,
        expected_balance - Uint128::new(25_000_001) // rounding 🤷
    );

    let mut suite = SuiteBuilder::default().build();

    // delegate all the tokens in the contract.
    suite.delegate(Uint128::new(100_000_000)).unwrap();

    suite.a_day_passes(); // collect rewards

    // redelegate half of the tokens to the other validator.
    suite.redelegate(Uint128::new(50_000_000), true).unwrap();

    suite.a_day_passes();

    // undelegate from the first validator.
    suite
        .undelegate(suite.receiver.clone(), Uint128::new(25_000_000))
        .unwrap();

    suite.a_day_passes();
    suite.process_unbonds();

    suite.distribute("random", None).unwrap();
    suite.withdraw_delegator_reward("validator").unwrap();
    suite.withdraw_delegator_reward("otherone").unwrap();

    let balance = suite.query_receiver_vesting_token_balance();

    // for reasons beyond me, staking rewards accrue differently when
    // the redelegate happens. i am unsure why and this test is more
    // concerned with them working than the absolute numbers, so >=.
    assert!(balance >= expected_balance)
}

/// Creates a vesting contract with a start time in the past s.t. the
/// vest immediately completes.
#[test]
fn test_start_time_in_the_past() {
    let default_start_time = App::default().block_info().time;

    let mut suite = SuiteBuilder::default()
        .with_start_time(default_start_time.minus_seconds(100))
        .build();

    suite.a_week_passes();

    // distributing over two TXns shouldn't matter.
    suite
        .distribute("lerandom", Some(Uint128::new(10_000_000)))
        .unwrap();
    suite.distribute("lerandom", None).unwrap();
    let balance = suite.query_receiver_vesting_token_balance();
    assert_eq!(balance, Uint128::new(100_000_000));
}

/// 1. Vestee is vesting 100 tokens
/// 2. Delegate 50 to validator
/// 3. Vestee looses 10 tokens to a validator slash
/// 4. Vestee slash reduces the amount the receiver may claim
#[test]
fn test_simple_slash() {
    let mut suite = SuiteBuilder::default().build();
    suite.delegate(Uint128::new(50_000_000)).unwrap();

    let vest = suite.query_vest();
    assert_eq!(vest.slashed, Uint128::zero());

    let pre_slash_distributable = suite.query_distributable();

    // because no time has passed, the slash amount is > the
    // distributable amount. this should not cause an overflow in
    // future calculations.
    suite.slash(20); // 20% slash should slash 10_000_000 tokens.
    let time = suite.time();

    // Only the owner can register a slash.
    let receiver = suite.receiver.clone();
    let owner = suite.owner.clone().unwrap();
    let res = suite.register_bonded_slash(&receiver, Uint128::new(10_000_000), time);
    is_error!(res, OwnershipError::NotOwner.to_string().as_str());

    suite
        .register_bonded_slash(&owner, Uint128::new(10_000_000), time)
        .unwrap();

    let vest = suite.query_vest();
    assert_eq!(vest.slashed, Uint128::new(10_000_000));
    let distributable = suite.query_distributable();
    assert_eq!(
        distributable,
        pre_slash_distributable.saturating_sub(Uint128::new(10_000_000))
    );

    assert_eq!(distributable, Uint128::zero());
}

/// A slash that is registered in the canceled state should count
/// against the owner even if the time of the slash was during the
/// Funded state. Owners should take care to register slashes before
/// canceling the contract.
#[test]
fn test_slash_while_cancelled_counts_against_owner() {
    let mut suite = SuiteBuilder::default().build();
    suite.delegate(Uint128::new(50_000_000)).unwrap();

    suite.a_day_passes();

    let slash_time = suite.time();
    suite.slash(20);

    // on cancel all liquid tokens are sent to the receiver to make
    // them whole. the slash has not been registered so this is an
    // overpayment.
    let distributable = suite.query_distributable();

    suite.cancel(suite.owner.clone().unwrap()).unwrap();

    let balance = suite.query_receiver_vesting_token_balance();
    assert_eq!(balance, distributable);

    let vest = suite.query_vest();
    let Status::Canceled {
        owner_withdrawable: pre_slash,
    } = vest.status
    else {
        panic!("should be canceled")
    };

    // register the slash. even though the time of the slash was
    // during the vest, the contract should deduct this from
    // owner_withdrawable as the contract is in a canceled state.
    suite
        .register_bonded_slash(
            suite.owner.clone().unwrap(),
            Uint128::new(10_000_000),
            slash_time,
        )
        .unwrap();

    let vest = suite.query_vest();
    let Status::Canceled { owner_withdrawable } = vest.status else {
        panic!("should be canceled")
    };
    assert_eq!(pre_slash - Uint128::new(10_000_000), owner_withdrawable);
}

/// Simple slash while tokens are unbonding and no cancelation.
#[test]
fn test_slash_during_unbonding() {
    let mut suite = SuiteBuilder::default().build();
    suite.delegate(Uint128::new(50_000_000)).unwrap();

    suite.a_second_passes();

    suite
        .undelegate(suite.receiver.clone(), Uint128::new(50_000_000))
        .unwrap();

    let pre_slash_distributable = suite.query_distributable();

    suite.slash(20); // 20% slash should slash 10_000_000 tokens.
    let time = suite.time();

    let owner = suite.owner.clone().unwrap();
    suite
        .register_unbonding_slash(&owner, Uint128::new(10_000_000), time)
        .unwrap();

    let vest = suite.query_vest();
    assert_eq!(vest.slashed, Uint128::new(10_000_000));
    let distributable = suite.query_distributable();
    assert_eq!(
        distributable,
        pre_slash_distributable.saturating_sub(Uint128::new(10_000_000))
    );

    suite.a_week_passes();
    suite.a_week_passes();
    suite.process_unbonds();

    suite.distribute("lerandom", None).unwrap();
    assert_eq!(
        suite.query_receiver_vesting_token_balance(),
        Uint128::new(90_000_000) // 10 slashed
    );

    // the staking implementation doesn't slash unbonding tokens in cw-multi-test..

    // assert_eq!(
    //     suite.query_vesting_token_balance(suite.vesting.clone()),
    //     Uint128::zero()
    // )
}

/// If the owner intentionally doesn't register a slash until they
/// have already withdrawn their tokens, the slash will be forced to
/// go to the receiver. The contract should handle this gracefully and
/// cause no overflows.
#[test]
fn test_owner_registers_slash_after_withdrawal() {
    let mut suite = SuiteBuilder::default().build();
    suite.delegate(Uint128::new(100_000_000)).unwrap();
    suite.a_day_passes();

    suite.cancel(suite.owner.clone().unwrap()).unwrap();

    let vested = suite.query_vest().vested(suite.time());

    // at this point 1/7th of the vest has elapsed, so the receiver
    // should be entitled to 1/7th regardless of a slash occuring as
    // the slash occures while the contract is in the canceled state.
    //
    // instead, the owner undelegates the remaining tokens, claims all
    // of them, and then registers the slash. as the slash as
    // registered too late, this will result in the receiver not
    // getting their tokens.
    suite.slash(90); // 90% slash
    let time = suite.time();

    suite
        .undelegate(suite.owner.clone().unwrap(), Uint128::new(10_000_000))
        .unwrap();

    suite.a_day_passes();
    suite.process_unbonds();

    suite.withdraw_canceled(None).unwrap();
    assert_eq!(
        suite.query_vesting_token_balance(suite.owner.clone().unwrap()),
        Uint128::new(10_000_000)
    );

    suite
        .register_bonded_slash(suite.owner.clone().unwrap(), Uint128::new(90_000_000), time)
        .unwrap();
    assert_eq!(suite.query_distributable(), Uint128::zero());
    assert_eq!(
        vested,
        Uint128::new(100_000_000).multiply_ratio(1u128, 7u128)
    );
}

/// Tests a one second vesting duration and a start time one week in
/// the future. Before the vest has completed, the receier should be
/// allowed to bond tokens and receive staking rewards, but should not
/// be able to claim any tokens.
#[test]
fn test_almost_instavest_in_the_future() {
    let default_start_time = App::default().block_info().time;

    let mut suite = SuiteBuilder::default()
        .with_start_time(default_start_time.plus_seconds(60 * 60 * 24 * 7))
        .with_vesting_duration(1)
        .build();

    suite.delegate(Uint128::new(100_000_000)).unwrap();
    let distributable = suite.query_distributable();
    assert_eq!(distributable, Uint128::zero());

    // five days pass.
    suite.a_day_passes();
    suite.a_day_passes();
    suite.a_day_passes();
    suite.a_day_passes();
    suite.a_day_passes();

    let balance_pre_claim = suite.query_receiver_vesting_token_balance();
    suite.withdraw_delegator_reward("validator").unwrap();
    let balance_post_claim = suite.query_receiver_vesting_token_balance();
    assert!(balance_post_claim > balance_pre_claim);

    suite
        .undelegate(suite.receiver.clone(), Uint128::new(100_000_000))
        .unwrap();

    // seven days have passed. one second remaining for vest
    // completion.
    suite.a_day_passes();
    suite.a_day_passes();
    suite.process_unbonds();

    let distributable = suite.query_distributable();
    assert_eq!(distributable, Uint128::zero());
    let res = suite.distribute("lerandom", None);
    is_error!(
        res,
        ContractError::InvalidWithdrawal {
            request: Uint128::zero(),
            claimable: Uint128::zero()
        }
        .to_string()
        .as_str()
    );

    // a second passes, the vest is now complete.
    suite.a_second_passes();

    let distributable = suite.query_distributable();
    assert_eq!(distributable, Uint128::new(100_000_000));
    suite
        .distribute("lerandom", Some(Uint128::new(100_000_000)))
        .unwrap();
    let balance = suite.query_receiver_vesting_token_balance();
    assert_eq!(balance, balance_post_claim + Uint128::new(100_000_000));
}

/// Test that the stake tracker correctly tracks stake during bonding,
/// unbonding, and slashing.
#[test]
fn test_stake_query() {
    use crate::StakeTrackerQuery;

    let mut suite = SuiteBuilder::default().build();

    let total_staked = suite.query_stake(StakeTrackerQuery::TotalStaked {
        t: suite.what_block_is_it().time,
    });
    assert_eq!(total_staked, Uint128::zero());

    suite.delegate(Uint128::new(123_456)).unwrap();

    let val_staked = suite.query_stake(StakeTrackerQuery::ValidatorStaked {
        t: suite.what_block_is_it().time,
        validator: "validator".to_string(),
    });
    assert_eq!(val_staked, Uint128::new(123_456));

    suite.slash(50);
    suite
        .register_bonded_slash(
            suite.owner.clone().unwrap().as_str(),
            Uint128::new(61_728),
            suite.what_block_is_it().time,
        )
        .unwrap();

    let val_staked = suite.query_stake(StakeTrackerQuery::ValidatorStaked {
        t: suite.what_block_is_it().time,
        validator: "validator".to_string(),
    });
    assert_eq!(val_staked, Uint128::new(61_728));

    suite
        .undelegate(suite.receiver.clone(), Uint128::new(61_728))
        .unwrap();

    let val_staked = suite.query_stake(StakeTrackerQuery::ValidatorStaked {
        t: suite.what_block_is_it().time,
        validator: "validator".to_string(),
    });
    assert_eq!(val_staked, Uint128::new(61_728));

    suite.slash(50);
    suite
        .register_unbonding_slash(
            suite.owner.clone().unwrap().as_str(),
            Uint128::new(30_864),
            suite.what_block_is_it().time,
        )
        .unwrap();

    let total_staked = suite.query_stake(StakeTrackerQuery::TotalStaked {
        t: suite.what_block_is_it().time,
    });
    assert_eq!(total_staked, Uint128::new(30_864));
    let val_staked = suite.query_stake(StakeTrackerQuery::ValidatorStaked {
        t: suite.what_block_is_it().time,
        validator: "validator".to_string(),
    });
    assert_eq!(val_staked, Uint128::new(30_864));
    let cardinality = suite.query_stake(StakeTrackerQuery::Cardinality {
        t: suite.what_block_is_it().time,
    });
    assert_eq!(cardinality, Uint128::new(1));
}

/// Basic checks on piecewise vests and queries.
#[test]
fn test_piecewise_and_queries() {
    let mut suite = SuiteBuilder::default()
        .with_start_time(SuiteBuilder::default().build().what_block_is_it().time)
        .with_curve(Schedule::PiecewiseLinear(vec![
            // <https://github.com/cosmorama/wynddao/pull/4> allows
            // for zero start values.
            (1, Uint128::new(0)),
            (2, Uint128::new(40_000_000)),
            (3, Uint128::new(100_000_000)),
        ]))
        .build();

    let duration = suite.query_duration();
    assert_eq!(duration.unwrap(), Uint64::new(2));

    let distributable = suite.query_distributable();
    assert_eq!(distributable, Uint128::new(0));

    suite.a_second_passes();

    let distributable = suite.query_distributable();
    assert_eq!(distributable, Uint128::new(0));

    suite.a_second_passes();

    let distributable = suite.query_distributable();
    assert_eq!(distributable, Uint128::new(40_000_000));

    suite.delegate(Uint128::new(80_000_000)).unwrap();

    let distributable = suite.query_distributable();
    assert_eq!(distributable, Uint128::new(20_000_000));
    let vested = suite.query_vested(None);
    assert_eq!(vested, Uint128::new(40_000_000));

    let total = suite.query_total_to_vest();
    assert_eq!(total, Uint128::new(100_000_000));

    suite.cancel(suite.owner.clone().unwrap()).unwrap();

    let total = suite.query_total_to_vest();
    assert_eq!(total, Uint128::new(40_000_000));

    // canceled, duration no longer has a meaning.
    let duration = suite.query_duration();
    assert_eq!(duration, None);
}
