use cosmwasm_schema::{cw_serde};
use cosmwasm_std::{Addr, Uint128};

#[cw_serde]
pub struct InstantiateMsg {
    // To determine voting power
    pub voting_contract: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    Receive(cw20::Cw20ReceiveMsg),
    FundNative {},
    ClaimCW20 { tokens: Option<Vec<String>> },
    ClaimNatives { denoms: Option<Vec<String>> },
    ClaimAll {},
}

#[cw_serde]
pub enum QueryMsg {
    TotalPower {},
    VotingContract {},
    NativeDenoms {},
    CW20Tokens {},
    NativeEntitlement { sender: String, denom: String },
    CW20Entitlement { sender: String, token: String },
    NativeEntitlements { sender: Addr },
    CW20Entitlements { sender: Addr },
}

#[cw_serde]
pub struct VotingContractResponse {
    // voting power contract being used
    pub contract: Addr,
    // height at which voting power is being determined
    pub distribution_height: u64,
}

#[cw_serde]
pub struct TotalPowerResponse {
    // total power at the distribution height
    pub total_power: Uint128,
}

#[cw_serde]
pub enum MigrateMsg {
    RedistributeUnclaimedFunds {
        distribution_height: u64,
    }
}

#[cw_serde]
pub struct DenomResponse {
    pub contract_balance: Uint128,
    pub denom: String,
}

#[cw_serde]
pub struct CW20Response {
    pub contract_balance: Uint128,
    pub token: String,
}

#[cw_serde]
pub struct NativeEntitlementResponse {
    pub amount: Uint128,
    pub denom: String,
}

#[cw_serde]
pub struct CW20EntitlementResponse {
    pub amount: Uint128,
    pub token_contract: Addr,
}

