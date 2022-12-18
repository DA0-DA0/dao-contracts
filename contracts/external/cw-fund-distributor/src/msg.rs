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
}

#[cw_serde]
pub enum QueryMsg {
    TotalPower {},
    VotingContract {},
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

