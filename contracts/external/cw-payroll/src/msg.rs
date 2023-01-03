use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint128;
use cw20::Cw20ReceiveMsg;
use cw_denom::CheckedDenom;
use wynd_utils::Curve;

#[cw_serde]
pub struct InstantiateMsg {
    pub admin: Option<String>,
    pub create_new_vesting_schedule_params: Option<VestingParams>,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Receive a cw20
    Receive(Cw20ReceiveMsg),
    /// Create a new vesting payment
    Create(VestingParams),
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
    Create(VestingParams),
}

#[cw_serde]
pub struct VestingParams {
    pub recipient: String,
    pub amount: Uint128,
    pub denom: CheckedDenom,
    pub vesting_schedule: Curve,
    // pub start_time: u64,
    // pub end_time: u64,
    pub title: Option<String>,
    pub description: Option<String>,
}

// TODO get vesting_payments by recipient
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(ConfigResponse)]
    Config {},
    #[returns(VestingPaymentResponse)]
    GetVestingPayment { id: u64 },
    #[returns(ListVestingPaymentsResponse)]
    ListVestingPayments {
        start: Option<u8>,
        limit: Option<u8>,
    },
}

#[cw_serde]
pub struct ConfigResponse {
    pub admin: String,
}

#[cw_serde]
pub struct VestingPaymentResponse {
    pub id: u64,
    pub recipient: String,
    pub amount: Uint128,
    pub claimed_amount: Uint128,
    pub denom: CheckedDenom,
    pub vesting_schedule: Curve,
    // pub start_time: u64,
    // pub end_time: u64,
    // pub paused_time: Option<u64>,
    // pub paused_duration: Option<u64>,
    /// Whether the payroll vesting_payment is currently paused
    pub paused: bool,
    /// Human readable title for this contract
    pub title: Option<String>,
    /// Human readable description for this payroll contract
    pub description: Option<String>,
}

#[cw_serde]
pub struct ListVestingPaymentsResponse {
    pub vesting_payments: Vec<VestingPaymentResponse>,
}
