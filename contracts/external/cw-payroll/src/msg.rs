use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint128;
use cw20::Cw20ReceiveMsg;
use cw_ownable::cw_ownable;

use crate::state::UncheckedVestingParams;

#[cw_serde]
pub struct InstantiateMsg {
    pub owner: Option<String>,
    pub params: UncheckedVestingParams,
}

#[cw_ownable]
#[cw_serde]
pub enum ExecuteMsg {
    /// Receive a cw20
    Receive(Cw20ReceiveMsg),
    /// Distribute unlocked vesting tokens
    Distribute {},
    /// Cancel vesting contract and return funds to owner (if configured)
    Cancel {},
    /// This is translated to a [MsgDelegate](https://github.com/cosmos/cosmos-sdk/blob/v0.40.0/proto/cosmos/staking/v1beta1/tx.proto#L81-L90).
    /// `delegator_address` is automatically filled with the current contract's address.
    /// Note: this only works with the native staking denom of a Cosmos chain
    Delegate {
        /// The validator to delegate to
        validator: String,
        /// The amount to delegate
        amount: Uint128,
    },
    /// This is translated to a [MsgBeginRedelegate](https://github.com/cosmos/cosmos-sdk/blob/v0.40.0/proto/cosmos/staking/v1beta1/tx.proto#L112-L121).
    /// `delegator_address` is automatically filled with the current contract's address.
    Redelegate {
        src_validator: String,
        dst_validator: String,
        amount: Uint128,
    },
    /// This is translated to a [MsgUndelegate](https://github.com/cosmos/cosmos-sdk/blob/v0.40.0/proto/cosmos/staking/v1beta1/tx.proto#L112-L121).
    /// `delegator_address` is automatically filled with the current contract's address.
    Undelegate {
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
    /// Funds a vesting contract with a cw20 token
    Fund {},
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Returns info about the vesting payment
    #[returns(crate::state::VestingPayment)]
    Info {},
    /// Returns info about the contract ownership, if set
    #[returns(::cw_ownable::Ownership<::cosmwasm_std::Addr>)]
    Ownership {},
    /// Returns the amount of funds that have vested at the current block
    #[returns(::cosmwasm_std::Uint128)]
    VestedAmount {},
}
