use cosmwasm_std::Addr;
use cw2::ContractVersion;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::state::Config;

/// Relevant state for the governance module.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct DumpStateResponse {
    /// The governance contract's config.
    pub config: Config,
    /// The governance contract's version.
    pub version: ContractVersion,
    /// The governance modules associated with the governance
    /// contract.
    pub governance_modules: Vec<Addr>,
    /// The voting module associated with the governance contract.
    pub voting_module: Addr,
}
