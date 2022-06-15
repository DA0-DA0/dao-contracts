use cosmwasm_std::{Addr, Binary, CosmosMsg, Empty};
use cw_utils::Duration;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cw_core_macros::voting_query;

use crate::state::Config;

/// Information about the admin of a contract.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Admin {
    /// A specific address.
    Address { addr: String },
    /// The core contract itself. The contract will fill this in
    /// while instantiation takes place.
    CoreContract {},
    /// No admin.
    None {},
}

/// Information needed to instantiate a proposal or voting module.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct InitialItem {
    /// The name of the item.
    pub key: String,
    /// The value the item will have at instantiation time.
    pub value: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct InstantiateMsg {
    /// Optional Admin with the ability to execute DAO messages directly
    /// Useful for building SubDAOs completely controlled by a parent DAO.
    pub admin: Option<String>,
    /// The name of the core contract.
    pub name: String,
    /// A description of the core contract.
    pub description: String,
    /// An image URL to describe the core module contract.
    pub image_url: Option<String>,

    /// If true the contract will automatically add received cw20
    /// tokens to its treasury.
    pub automatically_add_cw20s: bool,
    /// If true the contract will automatically add received cw721
    /// tokens to its treasury.
    pub automatically_add_cw721s: bool,

    /// Instantiate information for the core contract's voting
    /// power module.
    pub voting_module_instantiate_info: ModuleInstantiateInfo,
    /// Instantiate information for the core contract's
    /// proposal modules.
    pub proposal_modules_instantiate_info: Vec<ModuleInstantiateInfo>,

    /// Initial information for arbitrary contract addresses to be
    /// added to the items map. The key is the name of the item in the
    /// items map. The value is an enum that either uses an existing
    /// address or instantiates a new contract.
    pub initial_items: Option<Vec<InitialItem>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// Callable by the Admin, if one is configured.
    /// Executes messages in order.
    ExecuteAdminMsgs { msgs: Vec<CosmosMsg<Empty>> },
    /// Callable by proposal modules. The DAO will execute the
    /// messages in the hook in order.
    ExecuteProposalHook { msgs: Vec<CosmosMsg<Empty>> },
    /// Pauses the DAO for a set duration.
    /// When paused the DAO is unable to execute proposals
    Pause { duration: Duration },
    /// Executed when the contract receives a cw20 token. Depending on
    /// the contract's configuration the contract will automatically
    /// add the token to its treasury.
    Receive(cw20::Cw20ReceiveMsg),
    /// Executed when the contract receives a cw721 token. Depending
    /// on the contract's configuration the contract will
    /// automatically add the token to its treasury.
    ReceiveNft(cw721::Cw721ReceiveMsg),
    /// Removes an item from the governance contract's item map.
    RemoveItem { key: String },
    /// Adds an item to the governance contract's item map. If the
    /// item already exists the existing value is overriden. If the
    /// item does not exist a new item is added.
    SetItem { key: String, addr: String },
    /// Callable by the Admin, if one is configured, changes the
    /// current admin. Setting to None removes the Admin.
    UpdateAdmin { admin: Option<Addr> },
    /// Callable by the core contract. Replaces the current
    /// governance contract config with the provided config.
    UpdateConfig { config: Config },
    /// Updates the list of cw20 tokens this contract has registered.
    UpdateCw20List {
        to_add: Vec<String>,
        to_remove: Vec<String>,
    },
    /// Updates the list of cw721 tokens this contract has registered.
    UpdateCw721List {
        to_add: Vec<String>,
        to_remove: Vec<String>,
    },
    /// Updates the governance contract's governance modules. Module
    /// instantiate info in `to_add` is used to create new modules and
    /// install them.
    UpdateProposalModules {
        to_add: Vec<ModuleInstantiateInfo>,
        to_remove: Vec<String>,
    },
    /// Callable by the core contract. Replaces the current
    /// voting module with a new one instantiated by the governance
    /// contract.
    UpdateVotingModule { module: ModuleInstantiateInfo },
}

#[voting_query]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Get's the DAO's admin (if configured).
    Admin {},
    /// Gets the contract's config. Returns Config.
    Config {},
    /// Gets the token balance for each cw20 registered with the
    /// contract.
    Cw20Balances {
        start_at: Option<String>,
        limit: Option<u32>,
    },
    /// Lists the addresses of the cw20 tokens in this contract's
    /// treasury.
    Cw20TokenList {
        start_at: Option<String>,
        limit: Option<u32>,
    },
    /// Lists the addresses of the cw721 tokens in this contract's
    /// treasury.
    Cw721TokenList {
        start_at: Option<String>,
        limit: Option<u32>,
    },
    /// Dumps all of the core contract's state in a single
    /// query. Useful for frontends as performance for queries is more
    /// limited by network times than compute times. Returns
    /// `DumpStateResponse`.
    DumpState {},
    /// Gets the address associated with an item key.
    GetItem { key: String },
    /// Lists all of the items associted with the contract. For
    /// example, given the items `{ "group": "foo", "subdao": "bar"}`
    /// this query would return `[("group", "foo"), ("subdao",
    /// "bar")]`.
    ListItems {
        start_at: Option<String>,
        limit: Option<u32>,
    },
    /// Gets the proposal modules assocaited with the
    /// contract. Returns Vec<Addr>.
    ProposalModules {
        start_at: Option<String>,
        limit: Option<u32>,
    },
    /// Returns information about if the contract is currently paused.
    PauseInfo {},
    /// Gets the contract's voting module. Returns Addr.
    VotingModule {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct MigrateMsg {}
