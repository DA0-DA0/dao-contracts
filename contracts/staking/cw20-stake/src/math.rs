use std::{convert::TryInto, ops::Div};

use cosmwasm_std::{Uint128, Uint256};

/// Computes the amount to add to an address' staked balance when
/// staking.
///
/// # Arguments
///
/// * `staked_total` - The number of tokens that have been staked.
/// * `balance` - The number of tokens the contract has (staked_total + rewards).
/// * `sent` - The number of tokens the user has sent to be staked.
pub(crate) fn amount_to_stake(staked_total: Uint128, balance: Uint128, sent: Uint128) -> Uint128 {
    if staked_total.is_zero() || balance.is_zero() {
        sent
    } else {
        staked_total
            .full_mul(sent)
            .div(Uint256::from(balance))
            .try_into()
            .unwrap() // balance := staked_total + rewards
                      // => balance >= staked_total
                      // => staked_total / balance <= 1
                      // => staked_total * sent / balance <= sent
                      // => we can safely unwrap here as sent fits into a u128 by construction.
    }
}

/// Computes the number of tokens to return to an address when
/// claiming.
///
/// # Arguments
///
/// * `staked_total` - The number of tokens that have been staked.
/// * `balance` - The number of tokens the contract has (staked_total + rewards).
/// * `ask` - The number of tokens being claimed.
///
/// # Invariants
///
/// These must be checked by the caller. If checked, this function is
/// guarenteed not to panic.
///
/// 1. staked_total != 0.
/// 2. ask + balance <= 2^128
/// 3. ask <= staked_total
///
/// For information on the panic conditions for math, see:
/// <https://rust-lang.github.io/rfcs/0560-integer-overflow.html>
pub(crate) fn amount_to_claim(staked_total: Uint128, balance: Uint128, ask: Uint128) -> Uint128 {
    // we know that:
    //
    // 1. cw20's max supply is 2^128
    // 2. balance := staked_total + rewards
    //
    // for non-malicious inputs:
    //
    // 3. 1 => ask + balance <= 2^128
    // 4. ask <= staked_total
    // 5. staked_total != 0
    // 6. 4 => ask / staked_total <= 1
    // 7. 3 => balance <= 2^128
    // 8. 6 + 7 => ask / staked_total * balance <= 2^128
    //
    // which, as addition and division are communative, proves that
    // ask * balance / staked_total will fit into a 128 bit integer.
    ask.full_mul(balance)
        .div(Uint256::from(staked_total))
        .try_into()
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_amount_to_stake_no_overflow() {
        let sent = Uint128::new(2);
        let balance = Uint128::MAX - sent;

        let overflows_naively = sent.checked_mul(balance).is_err();
        assert!(overflows_naively);

        // will panic and fail the test if we've done this wrong.
        amount_to_stake(balance, balance, sent);
    }

    #[test]
    fn test_amount_to_stake_with_zeros() {
        let sent = Uint128::new(42);
        let balance = Uint128::zero();
        let amount = amount_to_stake(balance, balance, sent);
        assert_eq!(amount, sent);
    }

    #[test]
    fn test_amount_to_claim_no_overflow() {
        let ask = Uint128::new(2);
        let balance = Uint128::MAX - ask;

        let overflows_naively = ask.checked_mul(balance).is_err();
        assert!(overflows_naively);

        amount_to_claim(balance, balance, ask);
    }

    // check that our invariants are indeed invariants.

    #[test]
    #[should_panic(expected = "attempt to divide by zero")]
    fn test_amount_to_claim_invariant_one() {
        let ask = Uint128::new(2);
        let balance = Uint128::zero();

        amount_to_claim(balance, balance, ask);
    }

    #[test]
    #[should_panic(expected = "ConversionOverflowError")]
    fn test_amount_to_claim_invariant_two() {
        // Could end up in a situation like this if there are a lot of
        // rewards, but very few staked tokens.
        let ask = Uint128::new(2);
        let balance = Uint128::MAX;
        let staked_total = Uint128::new(1);

        amount_to_claim(staked_total, balance, ask);
    }

    #[test]
    #[should_panic(expected = "ConversionOverflowError")]
    fn test_amount_to_claim_invariant_three() {
        let ask = Uint128::new(2);
        let balance = Uint128::MAX;
        let staked_total = Uint128::new(1);

        amount_to_claim(staked_total, balance, ask);
    }
}
