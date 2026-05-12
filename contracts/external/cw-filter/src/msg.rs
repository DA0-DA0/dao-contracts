use cosmwasm_schema::{cw_serde, QueryResponses};

use cosmwasm_std::{Addr, CosmosMsg};
pub use cw_ownable::Ownership;
use cw_ownable::{cw_ownable_execute, cw_ownable_query};
use dao_interface::{proposal::InfoResponse, state::ModuleUpdate};

#[cw_serde]
pub struct InstantiateMsg {
    /// The address of the initial owner of the contract. Defaults to the
    /// sender.
    pub owner: Option<String>,
    /// The protobuf registry to use.
    pub protobuf_registry: Option<ModuleUpdate>,
}

#[cw_ownable_execute]
#[cw_serde]
pub enum ExecuteMsg {
    /// Update the protobuf registry.
    UpdateProtobufRegistry {
        protobuf_registry: Option<ModuleUpdate>,
    },
}

#[cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses)]
#[allow(clippy::large_enum_variant)]
pub enum QueryMsg {
    #[returns(InfoResponse)]
    Info {},
    #[returns(ProtobufRegistryResponse)]
    ProtobufRegistry {},
    #[returns(FilterResponse)]
    Filter {
        filter: serde_json::Value,
        msg: CosmosMsg,
    },
}

#[cw_serde]
pub struct MigrateMsg {}

// Response types

#[cw_serde]
pub struct ProtobufRegistryResponse {
    /// The address of the protobuf registry, if set.
    pub protobuf_registry: Option<Addr>,
}

#[cw_serde]
pub enum FilterResponse {
    Pass {},
    Fail {
        /// The reason for the filter failing.
        reason: String,
    },
    Fatal {
        /// The fatal reason for the filter failing.
        reason: String,
    },
}
