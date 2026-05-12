use cosmwasm_schema::cw_serde;
use cosmwasm_std::Coin;
use cw_storage_plus::{Item, Map};

#[cw_serde]
pub struct Config {
    /// Per-epoch budget — the amount distributed proportionally to weights
    /// each time `SampleGaugeMsgs` is queried.
    pub epoch_budget: Coin,
}

pub const CONFIG: Item<Config> = Item::new("config");

/// Set of valid option strings. Values are placeholders (we use this as a
/// set, not a map). The orchestrator passes options to `CheckOption` and
/// `SampleGaugeMsgs` as strings; we don't enforce any address shape here so
/// adapters can be used with non-address option identifiers (e.g. tags).
pub const OPTIONS: Map<&str, ()> = Map::new("options");
