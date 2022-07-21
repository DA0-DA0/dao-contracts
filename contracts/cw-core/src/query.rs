use cosmwasm_std::{Addr, Uint128};
use cw2::ContractVersion;
use cw_utils::Expiration;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::state::{Config, ProposalModule};

/// Relevant state for the governance module. Returned by the
/// `DumpState` query.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct DumpStateResponse {
    /// Optional DAO Admin
    pub admin: Addr,
    /// The governance contract's config.
    pub config: Config,
    // True if the contract is currently paused.
    pub pause_info: PauseInfoResponse,
    /// The governance contract's version.
    pub version: ContractVersion,
    /// The governance modules associated with the governance
    /// contract.
    pub proposal_modules: Vec<ProposalModule>,
    /// The voting module associated with the governance contract.
    pub voting_module: Addr,
}

/// Information about if the contract is currently paused.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum PauseInfoResponse {
    Paused { expiration: Expiration },
    Unpaused {},
}

/// Returned by the `GetItem` query.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct GetItemResponse {
    /// `None` if no item with the provided key was found, `Some`
    /// otherwise.
    pub item: Option<String>,
}

/// Returned by the `Cw20Balances` query.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct Cw20BalanceResponse {
    /// The address of the token.
    pub addr: Addr,
    /// The contract's balance.
    pub balance: Uint128,
}

/// Returned by the `AdminNomination` query.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct AdminNominationResponse {
    /// The currently nominated admin or None if no nomination is
    /// pending.
    pub nomination: Option<Addr>,
}
