use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{to_json_binary, Binary, StdResult, Storage, Timestamp, Uint128};
use cw_wormhole::Wormhole;

#[cfg(test)]
mod tests;

pub struct StakeTracker<'a> {
    /// staked(t) := the total number of native tokens staked &
    /// unbonding with validators at time t.
    total_staked: Wormhole<'a, (), Uint128>,
    /// validators(v, t) := the amount staked + amount unbonding with
    /// validator v at time t.
    ///
    /// deps.api.addr_validate does not validate validator addresses,
    /// so we're left with a string. in theory, as all of these
    /// functions are called only _on_ (un)delegation, their
    /// surrounding transactions should fail for invalid keys as the
    /// staking module ought to error. this is checked in
    /// `test_cw_vesting_staking` in
    /// `ci/integration-tests/src/tests/cw_vesting_test.rs`.
    validators: Wormhole<'a, String, Uint128>,
    /// cardinality(t) := the # of validators with staked and/or
    /// unbonding tokens at time t.
    cardinality: Wormhole<'a, (), u64>,
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum StakeTrackerQuery {
    #[returns(::cosmwasm_std::Uint128)]
    Cardinality { t: Timestamp },
    #[returns(::cosmwasm_std::Uint128)]
    TotalStaked { t: Timestamp },
    #[returns(::cosmwasm_std::Uint128)]
    ValidatorStaked { validator: String, t: Timestamp },
}

impl<'a> StakeTracker<'a> {
    pub const fn new(
        staked_prefix: &'a str,
        validator_prefix: &'a str,
        cardinality_prefix: &'a str,
    ) -> Self {
        Self {
            total_staked: Wormhole::new(staked_prefix),
            validators: Wormhole::new(validator_prefix),
            cardinality: Wormhole::new(cardinality_prefix),
        }
    }

    pub fn on_delegate(
        &self,
        storage: &mut dyn Storage,
        t: Timestamp,
        validator: String,
        amount: Uint128,
    ) -> StdResult<()> {
        self.total_staked
            .increment(storage, (), t.seconds(), amount)?;
        let old = self
            .validators
            .load(storage, validator.clone(), t.seconds())?
            .unwrap_or_default();
        if old.is_zero() && !amount.is_zero() {
            self.cardinality.increment(storage, (), t.seconds(), 1)?;
        }
        self.validators
            .increment(storage, validator, t.seconds(), amount)?;
        Ok(())
    }

    /// Makes note of a redelegation. Note, this only supports
    /// redelegation of tokens that can be _immediately_
    /// redelegated. The caller of this function should make a
    /// `Delegation { delegator, validator: src }` query and ensure
    /// that `amount <= resp.can_redelegate`.
    pub fn on_redelegate(
        &self,
        storage: &mut dyn Storage,
        t: Timestamp,
        src: String,
        dst: String,
        amount: Uint128,
    ) -> StdResult<()> {
        let new = self
            .validators
            .decrement(storage, src, t.seconds(), amount)?;
        if new.is_zero() {
            self.cardinality.decrement(storage, (), t.seconds(), 1)?;
        }
        let new = self
            .validators
            .increment(storage, dst, t.seconds(), amount)?;
        if new == amount {
            self.cardinality.increment(storage, (), t.seconds(), 1)?;
        }
        Ok(())
    }

    pub fn on_undelegate(
        &self,
        storage: &mut dyn Storage,
        t: Timestamp,
        validator: String,
        amount: Uint128,
        unbonding_duration_seconds: u64,
    ) -> StdResult<()> {
        self.total_staked.decrement(
            storage,
            (),
            t.seconds() + unbonding_duration_seconds,
            amount,
        )?;
        let new = self.validators.decrement(
            storage,
            validator,
            t.seconds() + unbonding_duration_seconds,
            amount,
        )?;
        if new.is_zero() && !amount.is_zero() {
            self.cardinality
                .decrement(storage, (), t.seconds() + unbonding_duration_seconds, 1)?;
        }
        Ok(())
    }

    /// Registers a slash of bonded tokens.
    ///
    /// Invariants:
    ///   1. amount is non-zero.
    ///   2. the slash did indeed occur.
    ///
    /// Checking that these invariants are true is the responsibility
    /// of the caller.
    pub fn on_bonded_slash(
        &self,
        storage: &mut dyn Storage,
        t: Timestamp,
        validator: String,
        amount: Uint128,
    ) -> StdResult<()> {
        enum Change {
            /// Increment by one at (time: u64).
            Inc(u64),
            /// Decrement by one at (time: u64).
            Dec(u64),
        }

        self.total_staked
            .decrement(storage, (), t.seconds(), amount)?;

        // tracks if the last value was non-zero after removing the
        // slash amount. invariant (2) lets us initialize this to true
        // as staked tokens are a prerequisite for slashing.
        let mut was_nonzero = true;
        // the set of times that the cardinality would have changed
        // had the slash event been known.
        let mut cardinality_changes = vec![];

        // visit the history, update values to include the slashed
        // amount, and make note of the changes to the cardinality
        // history needed.
        self.validators
            .update(storage, validator, t.seconds(), &mut |staked, time| {
                let new = staked - amount;
                if new.is_zero() && was_nonzero {
                    // the slash would have removed all staked tokens
                    // at `time` => decrement the cardinality at `time`.
                    cardinality_changes.push(Change::Dec(time));
                    was_nonzero = false;
                } else if !new.is_zero() && !was_nonzero {
                    // the staked amount (including the slash) was
                    // zero, and more tokens were staked, increment
                    // the cardinality.
                    cardinality_changes.push(Change::Inc(time));
                    was_nonzero = true;
                }
                new
            })?;

        // we can't do these updates as part of the `update` call
        // above as that would require two mutable references to
        // storage.
        for change in cardinality_changes {
            match change {
                Change::Inc(time) => self.cardinality.increment(storage, (), time, 1)?,
                Change::Dec(time) => self.cardinality.decrement(storage, (), time, 1)?,
            };
        }

        Ok(())
    }

    /// Registers a slash of unbonding tokens.
    ///
    /// Invariants:
    ///   1. amount is non-zero.
    ///   2. the slash did indeed occur.
    ///
    /// Checking that these invariants are true is the responsibility
    /// of the caller.
    pub fn on_unbonding_slash(
        &self,
        storage: &mut dyn Storage,
        t: Timestamp,
        validator: String,
        amount: Uint128,
    ) -> StdResult<()> {
        // invariant (2) provides that a slash did occur at time `t`,
        // and that the `amount` <= `total_unbonding`. As such, we
        // know at some time `t' > t`, total_staked, and
        // validator_staked are scheduled to decrease by an amount >=
        // `amount`. this means that we can safely use
        // `dangerously_update` as we are only adding an intermediate
        // step to reach a future value (`staked - total_unbonding`).

        self.total_staked
            .dangerously_update(storage, (), t.seconds(), &mut |v, _| v - amount)?;
        let new =
            self.validators
                .dangerously_update(storage, validator, t.seconds(), &mut |v, _| v - amount)?;
        if new.is_zero() {
            self.cardinality
                .dangerously_update(storage, (), t.seconds(), &mut |v, _| v - 1)?;
        }
        Ok(())
    }

    /// Gets the total number of bonded and unbonding tokens across
    /// all validators.
    pub fn total_staked(&self, storage: &dyn Storage, t: Timestamp) -> StdResult<Uint128> {
        self.total_staked
            .load(storage, (), t.seconds())
            .map(|v| v.unwrap_or_default())
    }

    /// Gets gets the number of tokens in the bonded or unbonding
    /// state for validator `v`.
    pub fn validator_staked(
        &self,
        storage: &dyn Storage,
        t: Timestamp,
        v: String,
    ) -> StdResult<Uint128> {
        self.validators
            .load(storage, v, t.seconds())
            .map(|v| v.unwrap_or_default())
    }

    /// Gets the number of validators for which there is a non-zero
    /// number of tokens in the bonding or unbonding state for.
    pub fn validator_cardinality(&self, storage: &dyn Storage, t: Timestamp) -> StdResult<u64> {
        self.cardinality
            .load(storage, (), t.seconds())
            .map(|v| v.unwrap_or_default())
    }

    /// Provides a query interface for contracts that embed this stake
    /// tracker and want to make its information part of their public
    /// API.
    pub fn query(&self, storage: &dyn Storage, msg: StakeTrackerQuery) -> StdResult<Binary> {
        match msg {
            StakeTrackerQuery::Cardinality { t } => to_json_binary(&Uint128::new(
                self.validator_cardinality(storage, t)?.into(),
            )),
            StakeTrackerQuery::TotalStaked { t } => to_json_binary(&self.total_staked(storage, t)?),
            StakeTrackerQuery::ValidatorStaked { validator, t } => {
                to_json_binary(&self.validator_staked(storage, t, validator)?)
            }
        }
    }
}
