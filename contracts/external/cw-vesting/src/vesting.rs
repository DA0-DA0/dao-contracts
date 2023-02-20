use std::cmp::min;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, CosmosMsg, DistributionMsg, StdResult, Storage, Timestamp, Uint128};
use cw_denom::CheckedDenom;
use cw_storage_plus::Item;
use wynd_utils::Curve;

use crate::{error::ContractError, stake_tracker::StakeTracker};

pub struct Payment<'a> {
    vesting: Item<'a, Vest>,
    staking: StakeTracker<'a>,
}

#[cw_serde]
pub struct Vest {
    /// vested(t), where t is seconds since start time.
    vested: Curve,
    start_time: Timestamp,

    pub status: Status,
    pub recipient: Addr,
    pub denom: CheckedDenom,
    pub claimed: Uint128,
    pub title: String,
    pub description: Option<String>,
}

#[cw_serde]
pub enum Status {
    Unfunded,
    Funded,
    Canceled {
        /// owner_withdrawable(t). This is monotonicly decreasing and
        /// will be zero once the owner has completed withdrawing
        /// their funds.
        owner_withdrawable: Uint128,
    },
}

#[cw_serde]
pub enum Schedule {
    /// Vests linearally from `0` to `total`.
    SaturatingLinear,
    /// Vests by linearally interpolating between the provided
    /// (timestamp, amount) points. The first amount must be zero and
    /// the last amount the total vesting amount. `timestamp` is a unix
    /// timestamp in SECONDS since epoch. Note that this differs from the
    /// CosmWasm `Timestamp` type which is normally specified in nanoseconds
    /// since epoch.
    PiecewiseLinear(Vec<(u64, Uint128)>),
}

pub struct VestInit {
    pub total: Uint128,
    pub schedule: Schedule,
    pub start_time: Timestamp,
    pub duration_seconds: u64,
    pub denom: CheckedDenom,
    pub recipient: Addr,
    pub title: String,
    pub description: Option<String>,
}

impl<'a> Payment<'a> {
    pub const fn new(
        vesting_prefix: &'a str,
        staked_prefix: &'a str,
        validator_prefix: &'a str,
        cardinality_prefix: &'a str,
    ) -> Self {
        Self {
            vesting: Item::new(vesting_prefix),
            staking: StakeTracker::new(staked_prefix, validator_prefix, cardinality_prefix),
        }
    }

    /// Validates its arguments and initializes the payment. Returns
    /// the underlying vest.
    pub fn initialize(
        &self,
        storage: &mut dyn Storage,
        init: VestInit,
    ) -> Result<Vest, ContractError> {
        let v = Vest::new(init)?;
        self.vesting.save(storage, &v)?;
        Ok(v)
    }

    pub fn get_vest(&self, storage: &dyn Storage) -> StdResult<Vest> {
        self.vesting.load(storage)
    }

    /// calculates the number of liquid tokens avaliable.
    fn liquid(&self, vesting: &Vest, staked: Uint128) -> Uint128 {
        match vesting.status {
            Status::Unfunded => Uint128::zero(),
            Status::Funded => vesting.total() - vesting.claimed - staked,
            Status::Canceled { owner_withdrawable } => {
                // On cancelation, all liquid funds are settled and
                // vesting.total() is set to the amount that has
                // vested so far. Then, the remaining staked tokens
                // are divided up between the owner and the vestee so
                // that the vestee will receive all of their vested
                // tokens. The following is then made true:
                //
                // staked = vesting_owned + owner_withdrawable
                // staked = (vesting.total - vesting.claimed) + owner_withdrawable
                //
                // staked - currently_staked = claimable, as those tokens
                // have unbonded and become avaliable and you can't
                // delegate in the cancelled state, so:
                //
                // claimable = (vesting.total - vesting.claimed) + owner_withdrawable - currently_staked
                //
                // Note that this is slightly simplified, in practice we
                // maintain:
                //
                // owner_withdrawable := owner.total - owner.claimed
                //
                // Where owner.total is the initial amount they were
                // entitled to.
                owner_withdrawable + (vesting.total() - vesting.claimed) - staked
            }
        }
    }

    /// Gets the current number tokens that may be distributed to the
    /// vestee.
    pub fn distributable(
        &self,
        storage: &dyn Storage,
        vesting: &Vest,
        t: Timestamp,
    ) -> StdResult<Uint128> {
        let staked = self.staking.total_staked(storage, t)?;

        let liquid = self.liquid(vesting, staked);
        let claimable = vesting.vested(t) - vesting.claimed;
        Ok(min(liquid, claimable))
    }

    /// Distributes vested tokens. If a specific amount is
    /// requested, that amount will be distributed, otherwise all
    /// tokens currently avaliable for distribution will be
    /// transfered.
    pub fn distribute(
        &self,
        storage: &mut dyn Storage,
        t: Timestamp,
        request: Option<Uint128>,
    ) -> Result<CosmosMsg, ContractError> {
        let vesting = self.vesting.load(storage)?;

        let distributable = self.distributable(storage, &vesting, t)?;
        let request = request.unwrap_or(distributable);

        let mut vesting = vesting;
        vesting.claimed += request;
        self.vesting.save(storage, &vesting)?;

        if request > distributable || request.is_zero() {
            Err(ContractError::InvalidWithdrawal {
                request,
                claimable: distributable,
            })
        } else {
            Ok(vesting
                .denom
                .get_transfer_to_message(&vesting.recipient, request)?)
        }
    }

    /// Cancels the vesting payment. The current amount vested becomes
    /// the total amount that will ever vest, and all staked tokens
    /// are unbonded. note that canceling does not impact already
    /// vested tokens.
    ///
    /// Upon canceling, the contract will use any liquid tokens in the
    /// contract to settle pending payments to the vestee, and then
    /// return the rest to the owner. If there are not enough liquid
    /// tokens to settle the vestee immediately, the vestee may
    /// distribute tokens as normal until they have received the
    /// amount of tokens they are entitled to. The owner may withdraw
    /// the remaining tokens via the `withdraw_canceled` method.
    pub fn cancel(
        &self,
        storage: &mut dyn Storage,
        t: Timestamp,
        owner: &Addr, // todo: also unbond field
    ) -> Result<Vec<CosmosMsg>, ContractError> {
        let mut vesting = self.vesting.load(storage)?;
        if matches!(vesting.status, Status::Canceled { .. }) {
            Err(ContractError::Cancelled {})
        } else {
            let staked = self.staking.total_staked(storage, t)?;

            // Use liquid tokens to settle vestee as much as possible
            // and return any remaining liquid funds to the owner.
            let liquid = self.liquid(&vesting, staked);
            let to_vestee = min(vesting.vested(t) - vesting.claimed, liquid);
            let to_owner = liquid - to_vestee;

            vesting.claimed += to_vestee;

            // After cancelation liquid funds are settled, and
            // the owners entitlement to the staked tokens is all
            // staked tokens that are not needed to settle the
            // vestee.
            let owner_outstanding = staked - (vesting.vested(t) - vesting.claimed);

            vesting.cancel(t, owner_outstanding);
            self.vesting.save(storage, &vesting)?;

            // Owner receives staking rewards
            let mut msgs = vec![DistributionMsg::SetWithdrawAddress {
                address: owner.to_string(),
            }
            .into()];

            if !to_owner.is_zero() {
                msgs.push(vesting.denom.get_transfer_to_message(owner, to_owner)?);
            }
            if !to_vestee.is_zero() {
                msgs.push(
                    vesting
                        .denom
                        .get_transfer_to_message(&vesting.recipient, to_vestee)?,
                );
            }

            Ok(msgs)
        }
    }

    pub fn withdraw_canceled(
        &self,
        storage: &mut dyn Storage,
        t: Timestamp,
        request: Option<Uint128>,
        owner: &Addr,
    ) -> Result<CosmosMsg, ContractError> {
        let vesting = self.vesting.load(storage)?;
        let staked = self.staking.total_staked(storage, t)?;
        if let Status::Canceled { owner_withdrawable } = vesting.status {
            let liquid = self.liquid(&vesting, staked);
            let claimable = min(liquid, owner_withdrawable);
            let request = request.unwrap_or(claimable);
            if request > claimable || request.is_zero() {
                Err(ContractError::InvalidWithdrawal { request, claimable })
            } else {
                let mut vesting = vesting;
                vesting.status = Status::Canceled {
                    owner_withdrawable: owner_withdrawable - request,
                };
                self.vesting.save(storage, &vesting)?;

                Ok(vesting.denom.get_transfer_to_message(owner, request)?)
            }
        } else {
            Err(ContractError::NotCancelled)
        }
    }

    pub fn on_undelegate(
        &self,
        storage: &mut dyn Storage,
        t: Timestamp,
        validator: String,
        amount: Uint128,
        unbonding_duration_seconds: u64,
    ) -> Result<(), ContractError> {
        self.staking
            .on_undelegate(storage, t, validator, amount, unbonding_duration_seconds)?;
        Ok(())
    }

    pub fn on_redelegate(
        &self,
        storage: &mut dyn Storage,
        t: Timestamp,
        src: String,
        dst: String,
        amount: Uint128,
    ) -> StdResult<()> {
        self.staking.on_redelegate(storage, t, src, dst, amount)?;
        Ok(())
    }

    pub fn on_delegate(
        &self,
        storage: &mut dyn Storage,
        t: Timestamp,
        validator: String,
        amount: Uint128,
    ) -> Result<(), ContractError> {
        self.staking.on_delegate(storage, t, validator, amount)?;
        Ok(())
    }

    pub fn set_funded(&self, storage: &mut dyn Storage) -> Result<(), ContractError> {
        let mut v = self.vesting.load(storage)?;
        debug_assert!(v.status == Status::Unfunded);
        v.status = Status::Funded;
        self.vesting.save(storage, &v)?;
        Ok(())
    }
}

impl Vest {
    pub fn new(init: VestInit) -> Result<Self, ContractError> {
        if init.total.is_zero() {
            Err(ContractError::ZeroVest {})
        } else {
            Ok(Self {
                claimed: Uint128::zero(),
                vested: init
                    .schedule
                    .into_curve(init.total, init.duration_seconds)?,
                start_time: init.start_time,
                denom: init.denom,
                recipient: init.recipient,
                status: Status::Unfunded,
                title: init.title,
                description: init.description,
            })
        }
    }

    /// Gets the total number of tokens that will vest as part of this
    /// payment.
    pub fn total(&self) -> Uint128 {
        Uint128::new(self.vested.range().1)
    }

    /// Gets the number of tokens that have vested at `time`.
    pub fn vested(&self, t: Timestamp) -> Uint128 {
        let elapsed =
            Timestamp::from_nanos(t.nanos().saturating_sub(self.start_time.nanos())).seconds();
        self.vested.value(elapsed)
    }

    /// Cancels the current vest. No additional tokens will vest after `t`.
    pub fn cancel(&mut self, t: Timestamp, owner_withdrawable: Uint128) {
        debug_assert!(!matches!(self.status, Status::Canceled { .. }));

        self.status = Status::Canceled { owner_withdrawable };
        self.vested = Curve::Constant { y: self.vested(t) };
    }
}

impl Schedule {
    /// The vesting schedule tracks vested(t), so for a curve to be
    /// valid:
    ///
    /// 1. it must start at 0,
    /// 2. it must end at total,
    /// 3. it must never decrease.
    ///
    /// A schedule is valid if `total` is zero: nothing will ever be
    /// paid out. Consumers should consider validating that `total` is
    /// non-zero.
    pub fn into_curve(self, total: Uint128, duration_seconds: u64) -> Result<Curve, ContractError> {
        let c = match self {
            Schedule::SaturatingLinear => {
                Curve::saturating_linear((0, 0), (duration_seconds, total.u128()))
            }
            Schedule::PiecewiseLinear(steps) => {
                Curve::PiecewiseLinear(wynd_utils::PiecewiseLinear { steps })
            }
        };
        c.validate_monotonic_increasing()?; // => max >= curve(t) \forall t
        let range = c.range();
        if range != (0, total.u128()) {
            return Err(ContractError::VestRange {
                min: Uint128::new(range.0),
                max: Uint128::new(range.1),
            });
        }
        Ok(c)
    }
}
