use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Timestamp, Uint128};
use cw20::Cw20ReceiveMsg;
use cw_denom::UncheckedDenom;
use cw_ownable::cw_ownable;

use crate::vesting::Schedule;

#[cw_serde]
pub struct InstantiateMsg {
    /// The optional owner address of the contract.
    pub owner: Option<String>,
    /// The receiver address of the vesting tokens.
    pub recipient: String,

    /// The a name or title for this payment.
    pub title: String,
    /// A description for the payment to provide more context.
    pub description: Option<String>,

    /// The total amount of tokens to be vested.
    pub total: Uint128,
    /// The type and denom of token being vested.
    pub denom: UncheckedDenom,

    /// The vesting schedule, can be either `SaturatingLinear` vesting
    /// (which vests evenly over time), or `PiecewiseLinear` which can
    /// represent a more complicated vesting schedule.
    pub schedule: Schedule,
    /// The time to start vesting, or None to start vesting when the
    /// contract is instantiated.
    pub start_time: Option<Timestamp>,
    /// The length of the vesting schedule in seconds.
    pub vesting_duration_seconds: u64,

    /// The unbonding duration for the chain this contract is deployed
    /// on. Smart contracts do not have access to this data so it must
    /// be provided by the caller.
    ///
    /// This value being too high will cause this contract to hold
    /// funds for longer than needed, this value being too low will
    /// reduce the quality of error messages and require additional
    /// external calculations with correct values to withdraw
    /// avaliable funds from the contract.
    pub unbonding_duration_seconds: u64,
}

#[cw_ownable]
#[cw_serde]
pub enum ExecuteMsg {
    /// Used to fund the contract with cw20 tokens when that is the
    /// token used for vesting. Otherwise, funds should be sent via
    /// the instantiate message.
    Receive(Cw20ReceiveMsg),
    /// Distribute vested tokens to the vest receiver. Anyone may call
    /// this method.
    Distribute {
        /// The amount of tokens to distribute. If none are specified
        /// all claimable tokens will be distributed.
        amount: Option<Uint128>,
    },
    /// Cancels the vesting payment. The current amount vested becomes
    /// the total amount that will ever vest, and all pending and
    /// future staking rewards from tokens staked by this contract
    /// will be sent to the owner. Tote that canceling does not impact
    /// already vested tokens.
    ///
    /// Upon canceling, the contract will use any liquid tokens in the
    /// contract to settle pending payments to the vestee, and then
    /// returns the rest to the owner. Staked tokens are then split
    /// between the owner and the vestee according to the number of
    /// tokens that the vestee is entitled to.
    ///
    /// The vestee will no longer receive staking rewards after
    /// cancelation, and may unbond and distribute (vested - claimed)
    /// tokens at their leisure. the owner will receive staking
    /// rewards and may unbond and withdraw (staked - (vested -
    /// claimed)) tokens at their leisure.
    Cancel {},
    /// This is translated to a
    /// [MsgDelegate](https://github.com/cosmos/cosmos-sdk/blob/v0.40.0/proto/cosmos/staking/v1beta1/tx.proto#L81-L90).
    /// `delegator_address` is automatically filled with the current
    /// contract's address.  Note: this only works with the native
    /// staking denom of a Cosmos chain.  Only callable by Vesting
    /// Payment Recipient.
    Delegate {
        /// The validator to delegate to.
        validator: String,
        /// The amount to delegate.
        amount: Uint128,
    },
    /// This is translated to a
    /// [MsgBeginRedelegate](https://github.com/cosmos/cosmos-sdk/blob/v0.40.0/proto/cosmos/staking/v1beta1/tx.proto#L96).
    /// `delegator_address` is automatically filled with the current
    /// contract's address.  Only callable by Vesting Payment
    /// Recipient.
    Redelegate {
        src_validator: String,
        dst_validator: String,
        amount: Uint128,
    },
    /// This is translated to a
    /// [MsgUndelegate](https://github.com/cosmos/cosmos-sdk/blob/v0.40.0/proto/cosmos/staking/v1beta1/tx.proto#L112-L121).
    /// `delegator_address` is automatically filled with the current
    /// contract's address.  Only callable by Vesting Payment
    /// Recipient.
    Undelegate {
        /// The validator to undelegate from
        validator: String,
        /// The amount to delegate
        amount: Uint128,
    },
    /// This is translated to a
    /// [MsgSetWithdrawAddress](https://github.com/cosmos/cosmos-sdk/blob/v0.42.4/proto/cosmos/distribution/v1beta1/tx.proto#L31-L37).
    /// `delegator_address` is automatically filled with the current
    /// contract's address.  Only callable by Vesting Payment
    /// Recipient.
    SetWithdrawAddress { address: String },
    /// This is translated to a
    /// [MsgWithdrawDelegatorReward](https://github.com/cosmos/cosmos-sdk/blob/v0.42.4/proto/cosmos/distribution/v1beta1/tx.proto#L42-L50).
    /// `delegator_address` is automatically filled with the current
    /// contract's address.  Only callable by Vesting Payment
    /// Recipient
    WithdrawDelegatorReward {
        /// The validator to claim rewards for.
        validator: String,
    },
    /// If the owner cancels a payment and there are not enough liquid
    /// tokens to settle the owner may become entitled to some number
    /// of staked tokens. They may then unbond those tokens and then
    /// call this method to return them.
    WithdrawCanceledPayment {
        /// The amount to withdraw.
        amount: Option<Uint128>,
    },
}

#[cw_serde]
pub enum ReceiveMsg {
    /// Funds a vesting contract with a cw20 token
    Fund {},
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Get the current ownership.
    #[returns(::cw_ownable::Ownership<::cosmwasm_std::Addr>)]
    Ownership {},
    /// Returns information about the vesting contract and the
    /// status of the payment.
    #[returns(crate::vesting::Vest)]
    Vest {},
    /// Returns the number of tokens currently claimable by the
    /// vestee. This is the minimum of the number of unstaked tokens
    /// in the contract, and the number of tokens that have been
    /// vested at time t.
    #[returns(::cosmwasm_std::Uint128)]
    Distributable {
        /// The time, as a unix timestamp in seconds, or none to use
        /// the current time.
        t: Option<u64>,
    },
}
