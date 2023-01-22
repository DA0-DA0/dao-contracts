use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map};
use cw_utils::Expiration;

/// block height for distribution snapshot
pub const DISTRIBUTION_HEIGHT: Item<u64> = Item::new("distribution_height");
/// period during which the contract can be funded
/// exclusive of the expiration block
pub const FUNDING_PERIOD_EXPIRATION: Item<Expiration> = Item::new("funding_period");
/// voting contract to determine the voting power
pub const VOTING_CONTRACT: Item<Addr> = Item::new("voting_contract");
/// total voting power at the distribution height
pub const TOTAL_POWER: Item<Uint128> = Item::new("total_power");

/// maps token address to the amount being distributed
pub const CW20_BALANCES: Map<Addr, Uint128> = Map::new("cw20_balances");
pub const NATIVE_BALANCES: Map<String, Uint128> = Map::new("native_balances");

/// maps (ADDRESS, TOKEN_ADDRESS) to amounts
/// that have been claimed by the address
pub const CW20_CLAIMS: Map<(Addr, Addr), Uint128> = Map::new("cw20_claims");
/// maps (ADDRESS, NATIVE_DENOM) to amounts
/// that have been claimed by the address
pub const NATIVE_CLAIMS: Map<(Addr, String), Uint128> = Map::new("native_claims");
