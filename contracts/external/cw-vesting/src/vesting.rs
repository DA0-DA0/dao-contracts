use std::cmp::min;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    to_binary, Addr, CosmosMsg, Env, StdResult, Storage, Timestamp, Uint128, WasmMsg,
};
use cw_denom::CheckedDenom;
use cw_storage_plus::Item;
use cw_wormhole::Wormhole;
use wynd_utils::Curve;

use crate::{error::ContractError, msg::ExecuteMsg};

pub struct Payment<'a> {
    vesting: Item<'a, Vest>,
    staking: Wormhole<'a, (), Uint128>,
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
    pub description: String,
}

#[cw_serde]
pub enum Status {
    Unfunded,
    Funded,
    Canceled {
        /// Number of tokens the owner may withdraw having canceled
        /// the vesting agreement.
        owner_withdrawable: Uint128,
    },
}

#[cw_serde]
pub enum Schedule {
    // Vests linearally from `0` to `total`.
    SaturatingLinear,
    // Vests by linearally interpolating between the provided
    // (timestamp, amount) points. The first amount must be zero and
    // the last `total`.
    PeacewiseLinear(Vec<(u64, Uint128)>),
}

pub struct VestInit {
    pub total: Uint128,
    pub schedule: Schedule,
    pub start_time: Timestamp,
    pub duration_seconds: u64,
    pub denom: CheckedDenom,
    pub recipient: Addr,
    pub title: String,
    pub description: String,
}

impl<'a> Payment<'a> {
    pub const fn new(vesting_prefix: &'a str, staking_prefix: &'a str) -> Self {
        Self {
            vesting: Item::new(vesting_prefix),
            staking: Wormhole::new(staking_prefix),
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
            Status::Unfunded | Status::Funded => vesting.total() - vesting.claimed - staked,
            Status::Canceled { owner_withdrawable } => {
                // on cancelation, all liquid funds are settled and
                // vesting.total() is set to the amount at that
                // time. then, the remaining staked tokens are divided up
                // between the owner and the veste so that the veste will
                // receive all of their vested tokens. the following is
                // then made true:
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
                // note that this is slightly simplified, in practice we
                // maintain:
                //
                // owner_withdrawable := owner.total - owner.claimed
                //
                // where owner.total is the initial amount they were
                // entitled to.
                owner_withdrawable + (vesting.total() - vesting.claimed) - staked
            }
        }
    }

    /// distributes vested tokens. if a specific amount is
    /// `request`ed, that amount will be distributed, otherwise all
    /// tokens currently avaliable for distribution will be
    /// transfered.
    pub fn distribute(
        &self,
        storage: &mut dyn Storage,
        t: &Timestamp,
        request: Option<Uint128>,
    ) -> Result<CosmosMsg, ContractError> {
        let vesting = self.vesting.load(storage)?;

        let staked = self
            .staking
            .load(storage, (), t.seconds())?
            .unwrap_or_default();

        let liquid = self.liquid(&vesting, staked);
        let claimable = min(liquid, vesting.vested(t) - vesting.claimed);
        let request = request.unwrap_or(claimable);

        let mut vesting = vesting;
        vesting.claimed += request;
        self.vesting.save(storage, &vesting)?;

        if request > claimable || request.is_zero() {
            Err(ContractError::InvalidWithdrawal { request, claimable })
        } else {
            Ok(vesting
                .denom
                .get_transfer_to_message(&vesting.recipient, request)?)
        }
    }

    /// cancels the vesting payment. the current amount vested becomes
    /// the total amount that will ever vest, and all pending and
    /// future staking rewards from tokens staked by this contract
    /// will be sent to the owner. note that canceling does not impact
    /// already vested tokens.
    ///
    /// upon canceling, the contract will use any liquid tokens in the
    /// contract to settle pending payments to the vestee, and then
    /// returns the rest to the owner. staked tokens are then split
    /// between the owner and the vestee according to the number of
    /// tokens that the vestee is entitled to.
    ///
    /// the vestee will no longer receive staking rewards after
    /// cancelation, and may unbond and distribute (vested - claimed)
    /// tokens at their leisure. the owner may unbond (staked -
    /// (vested - claimed)) tokens and withdraw them at their leisure.
    pub fn cancel(
        &self,
        storage: &mut dyn Storage,
        env: &Env,
        owner: &Addr,
    ) -> Result<Vec<CosmosMsg>, ContractError> {
        let t = &env.block.time;
        let mut vesting = self.vesting.load(storage)?;
        if matches!(vesting.status, Status::Canceled { .. }) {
            Err(ContractError::Cancelled {})
        } else {
            let staked = self
                .staking
                .load(storage, (), t.seconds())?
                .unwrap_or_default();

            // use liquid tokens to settle vestee as much as possible
            // and return any remaining liquid funds to the owner.
            let liquid = self.liquid(&vesting, staked);
            let to_vestee = min(vesting.vested(t) - vesting.claimed, liquid);
            let to_owner = liquid - to_vestee;

            vesting.claimed += to_vestee;

            // after cancelation liquid funds are settled, and
            // the owners entitlement to the staked tokens is all
            // staked tokens that are not needed to settle the
            // vestee.
            let owner_outstanding = staked - (vesting.vested(t) - vesting.claimed);

            vesting.cancel(t, owner_outstanding);
            self.vesting.save(storage, &vesting)?;

            // owner receives staking rewards
            let mut msgs = vec![WasmMsg::Execute {
                contract_addr: env.contract.address.to_string(),
                msg: to_binary(&ExecuteMsg::SetWithdrawAddress {
                    address: owner.to_string(),
                })?,
                funds: vec![],
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
        t: &Timestamp,
        request: Option<Uint128>,
        owner: &Addr,
    ) -> Result<CosmosMsg, ContractError> {
        let vesting = self.vesting.load(storage)?;
        let staked = self
            .staking
            .load(storage, (), t.seconds())?
            .unwrap_or_default();
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

    pub fn undelegate(
        &self,
        storage: &mut dyn Storage,
        t: &Timestamp,
        amount: Uint128,
        unbonding_duration_seconds: u64,
    ) -> Result<(), ContractError> {
        self.staking.decrement(
            storage,
            (),
            t.seconds() + unbonding_duration_seconds,
            amount,
        )?;
        Ok(())
    }

    pub fn delegate(
        &self,
        storage: &mut dyn Storage,
        t: &Timestamp,
        amount: Uint128,
    ) -> Result<(), ContractError> {
        self.staking.increment(storage, (), t.seconds(), amount)?;
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
    pub fn vested(&self, t: &Timestamp) -> Uint128 {
        let elapsed =
            Timestamp::from_nanos(t.nanos().saturating_sub(self.start_time.nanos())).seconds();
        self.vested.value(elapsed)
    }

    /// Cancels the current vest. No additional tokens will vest after `t`.
    pub fn cancel(&mut self, t: &Timestamp, owner_withdrawable: Uint128) {
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
            Schedule::PeacewiseLinear(steps) => {
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
