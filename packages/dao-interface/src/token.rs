use cosmwasm_schema::cw_serde;
use cosmwasm_std::Uint128;

// These are Cosmos Proto types used for Denom Metadata.
// We re-export them here for convenience.
pub use osmosis_std::types::cosmos::bank::v1beta1::{DenomUnit, Metadata};

use crate::state::ModuleInstantiateCallback;

#[cw_serde]
pub struct InitialBalance {
    pub amount: Uint128,
    pub address: String,
}

#[cw_serde]
pub struct NewDenomMetadata {
    /// The name of the token (e.g. "Cat Coin")
    pub name: String,
    /// The description of the token
    pub description: String,
    /// The ticker symbol of the token (e.g. "CAT")
    pub symbol: String,
    /// The unit commonly used in communication (e.g. "cat")
    pub display: String,
    /// Used define additional units of the token (e.g. "tiger")
    /// These must have an exponent larger than 0.
    pub additional_denom_units: Option<Vec<DenomUnit>>,
}

#[cw_serde]
pub struct NewTokenInfo {
    /// The code id of the cw-tokenfactory-issuer contract
    pub token_issuer_code_id: u64,
    /// The subdenom of the token to create, will also be used as an alias
    /// for the denom. The Token Factory denom will have the format of
    /// factory/{contract_address}/{subdenom}
    pub subdenom: String,
    /// Optional metadata for the token, this can additionally be set later.
    pub metadata: Option<NewDenomMetadata>,
    /// The initial balances to set for the token, cannot be empty.
    pub initial_balances: Vec<InitialBalance>,
    /// Optional balance to mint for the DAO.
    pub initial_dao_balance: Option<Uint128>,
}

#[cw_serde]
pub struct TokenFactoryCallback {
    pub denom: String,
    pub token_contract: Option<String>,
    pub module_instantiate_callback: Option<ModuleInstantiateCallback>,
}
