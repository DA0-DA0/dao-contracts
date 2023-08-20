use cosmwasm_std::{Addr, Empty};
use cw_storage_plus::{Item, Map};
use cw_utils::Expiration;
use dao_interface::state::{Config, ProposalModule};

/// The admin of the contract. Typically a DAO. The contract admin may
/// unilaterally execute messages on this contract.
///
/// In cases where no admin is wanted the admin should be set to the
/// contract itself. This will happen by default if no admin is
/// specified in `NominateAdmin` and instantiate messages.
pub const ADMIN: Item<Addr> = Item::new("admin");

/// A new admin that has been nominated by the current admin. The
/// nominated admin must accept the proposal before becoming the admin
/// themselves.
///
/// NOTE: If no admin is currently nominated this will not have a
/// value set. To load this value, use
/// `NOMINATED_ADMIN.may_load(deps.storage)`.
pub const NOMINATED_ADMIN: Item<Addr> = Item::new("nominated_admin");

/// The current configuration of the module.
pub const CONFIG: Item<Config> = Item::new("config_v2");

/// The time the DAO will unpause. Here be dragons: this is not set if
/// the DAO has never been paused.
pub const PAUSED: Item<Expiration> = Item::new("paused");

/// The voting module associated with this contract.
pub const VOTING_MODULE: Item<Addr> = Item::new("voting_module");

/// The proposal modules associated with this contract.
/// When we change the data format of this map, we update the key (previously "proposal_modules")
/// to create a new namespace for the changed state.
pub const PROPOSAL_MODULES: Map<Addr, ProposalModule> = Map::new("proposal_modules_v2");

/// The count of active proposal modules associated with this contract.
pub const ACTIVE_PROPOSAL_MODULE_COUNT: Item<u32> = Item::new("active_proposal_module_count");

/// The count of total proposal modules associated with this contract.
pub const TOTAL_PROPOSAL_MODULE_COUNT: Item<u32> = Item::new("total_proposal_module_count");

// General purpose KV store for DAO associated state.
pub const ITEMS: Map<String, String> = Map::new("items");

/// Set of cw20 tokens that have been registered with this contract's
/// treasury.
pub const CW20_LIST: Map<Addr, Empty> = Map::new("cw20s");
/// Set of cw721 tokens that have been registered with this contract's
/// treasury.
pub const CW721_LIST: Map<Addr, Empty> = Map::new("cw721s");

/// List of SubDAOs associated to this DAO. Each SubDAO has an optional charter.
pub const SUBDAO_LIST: Map<&Addr, Option<String>> = Map::new("sub_daos");
