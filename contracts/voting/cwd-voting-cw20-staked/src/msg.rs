use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Decimal, Uint128};
use cw20::Cw20Coin;
use cw20_base::msg::InstantiateMarketingInfo;
use cw_utils::Duration;

use cwd_macros::{active_query, info_query, token_query, voting_query};

#[cw_serde]
pub enum StakingInfo {
    Existing {
        staking_contract_address: String,
    },
    New {
        staking_code_id: u64,
        unstaking_duration: Option<Duration>,
    },
}

#[cw_serde]
#[allow(clippy::large_enum_variant)]
pub enum TokenInfo {
    Existing {
        address: String,
        staking_contract: StakingInfo,
    },
    New {
        code_id: u64,
        label: String,

        name: String,
        symbol: String,
        decimals: u8,
        initial_balances: Vec<Cw20Coin>,
        marketing: Option<InstantiateMarketingInfo>,
        staking_code_id: u64,
        unstaking_duration: Option<Duration>,
        initial_dao_balance: Option<Uint128>,
    },
}

#[cw_serde]
pub enum ActiveThreshold {
    AbsoluteCount { count: Uint128 },
    Percentage { percent: Decimal },
}

#[cw_serde]
pub struct InstantiateMsg {
    pub token_info: TokenInfo,
    pub active_threshold: Option<ActiveThreshold>,
}

#[cw_serde]
pub enum ExecuteMsg {
    UpdateActiveThreshold {
        new_threshold: Option<ActiveThreshold>,
    },
}

#[voting_query]
#[info_query]
#[token_query]
#[active_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(cosmwasm_std::Addr)]
    StakingContract {},
    #[returns(cosmwasm_std::Addr)]
    Dao {},
    #[returns(ActiveThresholdResponse)]
    ActiveThreshold {},
}

#[cw_serde]
pub struct ActiveThresholdResponse {
    pub active_threshold: Option<ActiveThreshold>,
}

#[cw_serde]
pub struct MigrateMsg {}
