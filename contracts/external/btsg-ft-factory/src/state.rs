use cw_storage_plus::Item;

use crate::msg::CreatingFanToken;

/// Temporarily holds data about the fan token being created that's needed in
/// reply so we can mint initial tokens and reset the minter.
pub const CREATING_FAN_TOKEN: Item<CreatingFanToken> = Item::new("cft");
