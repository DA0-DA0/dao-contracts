use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Timestamp, Uint128};
use cw_denom::CheckedDenom;
use cw_storage_plus::{Item, Map};

use crate::registration::Registration;

/// The configuration of the DAO registry.
pub const CONFIG: Item<Config> = Item::new("config");

/// Map address to its pending registration.
pub const PENDING_REGISTRATIONS: Map<Addr, Registration> = Map::new("pending_registrations");

/// Map address to its most recent registration.
pub const REGISTRATIONS: Map<Addr, Registration> = Map::new("registrations");

/// Map name to its most recent registered address.
pub const NAMES: Map<String, Addr> = Map::new("names");

#[cw_serde]
pub struct Config {
    /// The fee amount to register a DAO.
    pub fee_amount: Uint128,
    /// The fee denom to register a DAO.
    pub fee_denom: CheckedDenom,
    /// How long a registration lasts. For a new registration, this is the first
    /// expiration. For a renewal, this is the amount of time added to the
    /// current expiration.
    pub registration_period: Timestamp,
}
