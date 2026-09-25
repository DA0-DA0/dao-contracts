use cosmwasm_std::Addr;
use cw_hooks::Hooks;
use cw_storage_plus::Item;

/// The address of the DAO that instantiated this voting module.
pub const DAO: Item<Addr> = Item::new("dao");

/// Subscribers that receive `dao_hooks::stake::StakeChangedHookMsg` events
/// when delegations change. Managed via `AddHook` / `RemoveHook` (gated to
/// the DAO).
pub const HOOKS: Hooks = Hooks::new("hooks");
