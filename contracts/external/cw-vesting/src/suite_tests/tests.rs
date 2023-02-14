use cosmwasm_std::Uint128;

use crate::ContractError;

use super::{is_error, suite::SuiteBuilder};

#[test]
fn test_suite_instantiate() {
    SuiteBuilder::default().build();
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
    suite.withdraw_delegator_reward().unwrap();

    suite.cancel(suite.owner.clone().unwrap()).unwrap();

    suite.a_day_passes();

    // now that the vest is canceled, these rewards should go to the
    // owner.
    suite.withdraw_delegator_reward().unwrap();

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
