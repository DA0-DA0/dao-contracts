use cosmwasm_std::{StdResult, Storage, Timestamp, Uint128};
use cw_wormhole::Wormhole;

pub struct StakeTracker<'a> {
    /// staked(t) := the total number of native tokens staked with
    /// validators at time t.
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
            .increment(storage, validator, t.seconds(), amount)
    }

    /// Makes note of a redelegation. note, this only supports
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
        self.validators
            .decrement(storage, src, t.seconds(), amount)?;
        self.validators.increment(storage, dst, t.seconds(), amount)
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
        self.validators.decrement(
            storage,
            validator.clone(),
            t.seconds() + unbonding_duration_seconds,
            amount,
        )?;
        let new = self
            .validators
            .load(storage, validator, t.seconds() + unbonding_duration_seconds)?
            .expect("decrement should have errored on missing delegation");
        if new.is_zero() && !amount.is_zero() {
            self.cardinality
                .decrement(storage, (), t.seconds() + unbonding_duration_seconds, 1)?;
        }
        Ok(())
    }

    pub fn total_staked(&self, storage: &dyn Storage, t: Timestamp) -> StdResult<Uint128> {
        self.total_staked
            .load(storage, (), t.seconds())
            .map(|v| v.unwrap_or_default())
    }

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

    pub fn validator_cardinality(&self, storage: &dyn Storage, t: Timestamp) -> StdResult<u64> {
        self.cardinality
            .load(storage, (), t.seconds())
            .map(|v| v.unwrap_or_default())
    }
}
