use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint128;
use cw20::Cw20ReceiveMsg;

use crate::state::{Vest, Config};

#[cw_serde]
pub struct InstantiateMsg {
    pub owner: Option<String>,
    pub manager: Option<String>,
    pub token_address: String,
    pub stake_address: String,
    pub schedules: Vec<Schedule>
}

#[cw_serde]
pub struct Schedule {
    pub address: String,
    pub vests: Vec<Vest>,
}

#[cw_serde]
pub enum ExecuteMsg {
    Receive(Cw20ReceiveMsg),
    Vest {},
    Claim {},
    UpdateConfig {
        owner: Option<String>,
        manager: Option<String>,
    },
}

#[cw_serde]
pub enum ReceiveMsg {
    Fund {},
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(GetVestingStatusAtHeightResponse)]
    GetVestingStatusAtHeight {
        address: String,
        height: Option<u64>,
    },
    #[returns(GetFundingStatusAtHeightResponse)]
    GetFundingStatusAtHeight { height: Option<u64> },
    #[returns(Config)]
    GetConfig {},
}

#[cw_serde]
pub struct GetVestingStatusAtHeightResponse {
    pub vested_claimed: Uint128,
    pub vested_unstaked: Uint128,
    pub vested_unstaking: Uint128,
    pub vested_staked: Uint128,
    pub unvested_staked: Uint128,
    pub height: u64,
}

#[cw_serde]
pub enum MigrateMsg {
    FromBeta { manager: Option<String> },
    FromCompatible {},
}

#[cw_serde]
pub struct GetFundingStatusAtHeightResponse {
    pub activated: bool,
    pub height: u64,
}
