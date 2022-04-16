use cosmwasm_std::{Decimal, Uint128};
use cw20::Cw20Coin;
use cw20_base::msg::InstantiateMarketingInfo;
use cw_core_macros::{active_query, token_query, voting_query};
use cw_utils::Duration;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum StakingInfo {
    Existing {
        staking_contract_address: String,
    },
    New {
        staking_code_id: u64,
        unstaking_duration: Option<Duration>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ActiveThreshold {
    AbsoluteCount { count: Uint128 },
    Percentage { percent: Decimal },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub token_info: TokenInfo,
    pub active_threshold: Option<ActiveThreshold>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {}

#[voting_query]
#[token_query]
#[active_query]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    StakingContract {},
    Dao {},
}
