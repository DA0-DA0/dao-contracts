use cosmwasm_std::Uint128;
use cw_controllers::Hooks;
use cw_storage_plus::Item;

// Transfer/send contract hooks.
pub const HOOKS: Hooks = Hooks::new("hooks");

// Total supply cap set on instantiate if minting is allowed. If the owner
// decides to clear the minter and then add a minter later, this cap is restored
// to ensure the cap is preserved even when the minter is removed.
pub const CAP: Item<Option<Uint128>> = Item::new("cap");
