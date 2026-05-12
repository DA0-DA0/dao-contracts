use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{CustomMsg, Deps, Env, MessageInfo};
use cw721::error::Cw721ContractError;
use cw721::traits::{Contains, Cw721CustomMsg, Cw721State, StateFactory};

#[cw_serde]
pub struct MetadataExt {
    /// Optional on-chain role for this member, can be used by other contracts to enforce permissions
    pub role: Option<String>,
    /// The voting weight of this role
    pub weight: u64,
}
impl Cw721State for MetadataExt {}
impl Cw721CustomMsg for MetadataExt {}

/// `Contains` is required by `Cw721Query` so callers can ask "does this NFT have these
/// traits/metadata". For roles, an NFT contains another when role and weight both match.
impl Contains for MetadataExt {
    fn contains(&self, other: &Self) -> bool {
        self == other
    }
}

/// `MetadataExt` doubles as the cw721 NFT extension *state* and *msg*. The msg-to-state
/// conversion is identity — clone the msg into state with no validation.
impl StateFactory<MetadataExt> for MetadataExt {
    fn create(
        &self,
        _deps: Deps,
        _env: &Env,
        _info: Option<&MessageInfo>,
        _current: Option<&MetadataExt>,
    ) -> Result<MetadataExt, Cw721ContractError> {
        Ok(self.clone())
    }

    fn validate(
        &self,
        _deps: Deps,
        _env: &Env,
        _info: Option<&MessageInfo>,
        _current: Option<&MetadataExt>,
    ) -> Result<(), Cw721ContractError> {
        Ok(())
    }
}

#[cw_serde]
pub enum ExecuteExt {
    /// Add a new hook to be informed of all membership changes.
    /// Must be called by Admin
    AddHook { addr: String },
    /// Remove a hook. Must be called by Admin
    RemoveHook { addr: String },
    /// Update the token_uri for a particular NFT. Must be called by minter / admin
    UpdateTokenUri {
        token_id: String,
        token_uri: Option<String>,
    },
    /// Updates the voting weight of a token. Must be called by minter / admin
    UpdateTokenWeight { token_id: String, weight: u64 },
    /// Udates the role of a token. Must be called by minter / admin
    UpdateTokenRole {
        token_id: String,
        role: Option<String>,
    },
}
impl CustomMsg for ExecuteExt {}
impl Cw721CustomMsg for ExecuteExt {}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryExt {
    /// Total weight at a given height
    #[returns(cw4::TotalWeightResponse)]
    TotalWeight { at_height: Option<u64> },
    /// Returns a list of Members
    #[returns(cw4::MemberListResponse)]
    ListMembers {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// Returns the weight of a certain member
    #[returns(cw4::MemberResponse)]
    Member {
        addr: String,
        at_height: Option<u64>,
    },
    /// Shows all registered hooks.
    #[returns(cw_controllers::HooksResponse)]
    Hooks {},
}
impl CustomMsg for QueryExt {}
impl Cw721CustomMsg for QueryExt {}
