use cosmwasm_std::Uint128;
use cw2::ContractVersion;
use cw_core_macros::{active_query, token_query, voting_query};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[token_query]
#[voting_query]
#[active_query]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Query {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct VotingPowerAtHeightResponse {
    pub power: Uint128,
    pub height: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct TotalPowerAtHeightResponse {
    pub power: Uint128,
    pub height: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InfoResponse {
    pub info: ContractVersion,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct IsActiveResponse {
    pub active: bool,
}

mod tests {
    /// Make sure the enum has all of the fields we expect. This will
    /// fail to compile if not.
    #[test]
    fn test_macro_expansion() {
        use cw_core_macros::{active_query, token_query, voting_query};
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
