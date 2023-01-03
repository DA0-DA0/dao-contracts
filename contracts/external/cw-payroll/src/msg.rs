use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint128;
use cw20::Cw20ReceiveMsg;
use cw_denom::CheckedDenom;
use wynd_utils::Curve;
// so that consumers don't need a cw_ownable dependency to consume
// this contract's queries.
pub use cw_ownable::Ownership;

use cw_ownable::cw_ownable;

use crate::state::UncheckedVestingParams;

#[cw_serde]
pub struct InstantiateMsg {
    pub owner: Option<String>,
    pub create_new_vesting_schedule_params: Option<UncheckedVestingParams>,
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
    /// Pause the vesting contract
    Pause { id: u64 },
    /// Resume the vesting schedule
    Resume { id: u64 },
    /// Cancel vesting contract and return funds to owner (if configured)
    Cancel { id: u64 },
    /// Delegate vested native tokens
    Delegate {},
    /// Undelegate vested native tokens
    Undelegate {},
    /// Redelegate vested native tokens
    Redelgate {},
    /// Withdraw rewards from staked native tokens
    WithdrawRewards {},
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
