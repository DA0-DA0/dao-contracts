use cosmwasm_std::{Binary, CosmosMsg, Empty};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cw_governance_macros::voting_query;

use crate::state::Config;

/// Information about the admin of a contract.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Admin {
    /// A specific address.
    Address { addr: String },
    /// The governance contract itself. The contract will fill this in
    /// while instantiation takes place.
    GovernanceContract {},
    /// No admin.
    None {},
}

/// Information needed to instantiate a governance or voting module.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ModuleInstantiateInfo {
    /// Code ID of the contract to be instantiated.
    pub code_id: u64,
    /// Instantiate message to be used to create the contract.
    pub msg: Binary,
    /// Admin of the instantiated contract.
    pub admin: Admin,
    /// Label for the instantiated contract.
    pub label: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    /// The name of the governance contract.
    pub name: String,
    /// A description of the governance contract.
    pub description: String,
    /// An image URL to describe the governance module contract.
    pub image_url: Option<String>,

    /// Instantiate information for the governance contract's voting
    /// power module.
    pub voting_module_instantiate_info: ModuleInstantiateInfo,
    /// Instantiate information for the governance contract's
    /// governance modules.
    pub governance_modules_instantiate_info: Vec<ModuleInstantiateInfo>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// Callable by governance modules. The DAO will execute the
    /// messages in the hook in order.
    ExecuteProposalHook { msgs: Vec<CosmosMsg<Empty>> },
    /// Callable by the governance contract. Replaces the current
    /// governance contract config with the provided config.
    UpdateConfig { config: Config },
    /// Callable by the governance contract. Replaces the current
    /// voting module with a new one instantiated by the governance
    /// contract.
    UpdateVotingModule { module: ModuleInstantiateInfo },
    /// Updates the governance contract's governance modules. Module
    /// instantiate info in `to_add` is used to create new modules and
    /// install them.
    UpdateGovernanceModules {
        to_add: Vec<ModuleInstantiateInfo>,
        to_remove: Vec<String>,
    },
    /// Adds an item to the governance contract's item map. If the
    /// item already exists the existing value is overriden. If the
    /// item does not exist a new item is added.
    SetItem { key: String, addr: String },
    /// Removes an item from the governance contract's item map.
    RemoveItem { key: String },
}

#[voting_query]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Gets the contract's config. Returns Config.
    Config {},
    /// Gets the contract's voting module. Returns Addr.
    VotingModule {},
    /// Gets the governance modules assocaited with the
    /// contract. Returns Vec<Addr>.
    GovernanceModules {
        start_at: Option<String>,
        limit: Option<u64>,
    },
    /// Dumps all of the governance contract's state in a single
    /// query. Useful for frontends as performance for queries is more
    /// limited by network times than compute times. Returns
    /// `DumpStateResponse`.
    DumpState {},
    GetItem {
        key: String,
    },
    ListItems {
        start_at: Option<String>,
        limit: Option<u64>,
    },
}
