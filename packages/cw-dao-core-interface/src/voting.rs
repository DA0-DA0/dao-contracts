use cosmwasm_std::Uint128;
use cw2::ContractVersion;
use cw_dao_core_macros::{active_query, token_query, voting_query};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The query message interface that the cw-dao-core contract
/// implements.
#[voting_query]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Query {}

/// The query enum that voting modules may implement. The token and
/// active queries are optional. Callers should handle that
/// gracefully.
#[voting_query]
#[token_query]
#[active_query]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum VotingModuleQuery {}

/// The response type for a `VotingPowerAtHeight` query.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct VotingPowerAtHeightResponse {
    pub power: Uint128,
    pub height: u64,
}

/// The response type for a `TotalPowerAtHeight` query.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct TotalPowerAtHeightResponse {
    pub power: Uint128,
    pub height: u64,
}

/// The response type for an `Info` query.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InfoResponse {
    pub info: ContractVersion,
}

/// The response type for an `IsActive` query. Voting modules may have
/// conditions for them to be active. For example, a staking voting
/// module may require that some percentage of the total supply is
/// staked for voting to begin.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct IsActiveResponse {
    pub active: bool,
}

mod tests {
    /// Make sure the enum has all of the fields we expect. This will
    /// fail to compile if not.
    #[test]
    fn test_macro_expansion() {
        use cw_dao_core_macros::{active_query, token_query, voting_query};
        use schemars::JsonSchema;
        use serde::{Deserialize, Serialize};

        #[token_query]
        #[voting_query]
        #[active_query]
        #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
        #[serde(rename_all = "snake_case")]
        enum Query {}

        let query = Query::TokenContract {};

        match query {
            Query::TokenContract {} => (),
            Query::VotingPowerAtHeight { .. } => (),
            Query::TotalPowerAtHeight { .. } => (),
            Query::IsActive {} => (),
            Query::Info {} => (),
        }
    }
}
