use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, DepsMut, Order, Response, StdResult, Timestamp, Uint128};
use cw_controllers::Admin;
use cw_storage_plus::{Item, Map};
use cw_utils::maybe_addr;

use crate::msg::MigrateMsg;
use crate::state::{CONFIG, STAKED_BALANCES, STAKED_TOTAL};
use crate::ContractError;

pub const TWO_WEEKS: u64 = 86400 * 14;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct WyndexConfig {
    /// address of cw20 contract token to stake
    pub cw20_contract: Addr,
    /// address that instantiated the contract
    pub instantiator: Addr,
    pub tokens_per_power: Uint128,
    pub min_bond: Uint128,
    /// configured unbonding periods in seconds
    pub unbonding_periods: Vec<u64>,
    /// the maximum number of distributions that can be created
    pub max_distributions: u32,
}

#[derive(Default, Serialize, Deserialize)]
pub struct TokenInfo {
    // how many tokens are fully bonded
    pub staked: Uint128,
    // how many tokens are unbounded and awaiting claim
    pub unbonding: Uint128,
}

#[derive(Default, Serialize, Deserialize)]
pub struct BondingInfo {
    /// the amount of staked tokens which are not locked
    stake: Uint128,
    /// Vec of locked_tokens sorted by expiry timestamp
    locked_tokens: Vec<(Timestamp, Uint128)>,
}

pub fn migrate(mut deps: DepsMut, msg: MigrateMsg) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // this line is crucial, new storage item must be named same as old
    let new_storage: Item<WyndexConfig> = Item::new("config");
    let new_config = WyndexConfig {
        cw20_contract: config.token_address,
        instantiator: deps.api.addr_validate(&msg.pool_contract)?,
        tokens_per_power: msg.tokens_per_power,
        min_bond: msg.min_bond,
        unbonding_periods: msg.unbonding_periods,
        max_distributions: msg.max_distributions,
    };
    new_storage.save(deps.storage, &new_config)?;

    let new_admin: Admin = Admin::new("admin");
    let admin_address = maybe_addr(deps.api, msg.new_admin.clone())?;
    new_admin.set(deps.branch(), admin_address)?;

    let new_storage: Item<TokenInfo> = Item::new("total_staked");
    let staked_total = STAKED_TOTAL.load(deps.storage)?;
    new_storage.save(
        deps.storage,
        &TokenInfo {
            staked: staked_total,
            unbonding: Uint128::zero(),
        },
    )?;

    let new_storage: Map<(&Addr, u64), BondingInfo> = Map::new("stake");
    let new_stakes = STAKED_BALANCES
        .range(deps.as_ref().storage, None, None, Order::Ascending)
        .map(|stake| {
            let (addr, amount) = stake?;
            let bonding_info = BondingInfo {
                stake: amount,
                locked_tokens: vec![],
            };
            Ok(((addr, TWO_WEEKS), bonding_info))
        })
        .collect::<StdResult<Vec<((Addr, u64), BondingInfo)>>>()?;
    for ((addr, unbonding_period), bonding_info) in new_stakes {
        new_storage.save(deps.storage, (&addr, unbonding_period), &bonding_info)?;
    }

    Ok(Response::new())
}
