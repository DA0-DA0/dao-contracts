use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map};

/// The admin of this contract.
pub const ADMIN: Item<Option<Addr>> = Item::new("admin");
/// The height at which voting power is being determined.
pub const DISTRIBUTION_HEIGHT: Item<u64> = Item::new("distribution_height");
/// The total power at the height at which voting power is being
/// determined.
pub const TOTAL_POWER: Item<Uint128> = Item::new("total_power");
/// The voting contract being used.
pub const VOTING_CONTRACT: Item<Addr> = Item::new("voting_contract");

/// The cw20 tokens being distributed by this contract. Maps token
/// address to the amount of tokens being distributed.
pub const CW20S: Map<Addr, Uint128> = Map::new("cw20s");
/// The native tokens being distrbiuted by this contract. Maps native
/// denoms to the amount of tokens being distributed.
pub const NATIVES: Map<String, Uint128> = Map::new("natives");

/// Maps (ADDRESS, TOKEN_ADDRESS) to the amount of tokens that have
/// been claimed by the address.
pub const CW20_CLAIMS: Map<(Addr, Addr), Uint128> = Map::new("cw20_claims");
/// Maps (ADDRESS, NATIVE_DENOM) to the amount of tokens that have
/// been claimed by the address.
pub const NATIVE_CLAIMS: Map<(Addr, String), Uint128> = Map::new("native_claims");
