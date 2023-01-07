use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint128;
use cw20::Cw20ReceiveMsg;
use cw_ownable::cw_ownable;

use crate::state::UncheckedVestingParams;

#[cw_serde]
pub struct InstantiateMsg {
    pub owner: Option<String>,
}

#[cw_ownable]
#[cw_serde]
pub enum ExecuteMsg {
    /// Receive a cw20
    Receive(Cw20ReceiveMsg),
    /// Create a new vesting payment
    Create(UncheckedVestingParams),
    /// Distribute unlocked vesting tokens
    Distribute { id: u64 },
    /// Cancel vesting contract and return funds to owner (if configured)
    Cancel { id: u64 },
    /// This is translated to a [MsgDelegate](https://github.com/cosmos/cosmos-sdk/blob/v0.40.0/proto/cosmos/staking/v1beta1/tx.proto#L81-L90).
    /// `delegator_address` is automatically filled with the current contract's address.
    /// Note: this only works with the native staking denom of a Cosmos chain
    Delegate {
        /// The ID for the vesting payment
        vesting_payment_id: u64,
        /// The validator to delegate to
        validator: String,
        /// The amount to delegate
        amount: Uint128,
    },
    /// This is translated to a [MsgUndelegate](https://github.com/cosmos/cosmos-sdk/blob/v0.40.0/proto/cosmos/staking/v1beta1/tx.proto#L112-L121).
    /// `delegator_address` is automatically filled with the current contract's address.
    Undelegate {
        /// The ID for the vesting payment
        vesting_payment_id: u64,
        /// The validator to undelegate from
        validator: String,
        /// The amount to delegate
        amount: Uint128,
    },
    /// This is translated to a [[MsgWithdrawDelegatorReward](https://github.com/cosmos/cosmos-sdk/blob/v0.42.4/proto/cosmos/distribution/v1beta1/tx.proto#L42-L50).
    /// `delegator_address` is automatically filled with the current contract's address.
    WithdrawDelegatorReward {
        /// The `validator_address` to claim rewards for
        validator: String,
    },
}

// Receiver setup
#[cw_serde]
pub enum ReceiveMsg {
    Create(UncheckedVestingParams),
}

// TODO get vesting_payments by recipient
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(crate::state::VestingPayment)]
    GetVestingPayment { id: u64 },
    #[returns(Vec<crate::state::VestingPayment>)]
    ListVestingPayments {
        start_after: Option<u64>,
        limit: Option<u32>,
    },
    #[returns(::cw_ownable::Ownership<::cosmwasm_std::Addr>)]
    Ownership {},
}
