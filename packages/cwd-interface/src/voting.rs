use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint128;
use cw2::ContractVersion;
use cwd_macros::{active_query, token_query, voting_module_query};

#[token_query]
#[voting_module_query]
#[active_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum Query {}

#[cw_serde]
pub struct VotingPowerAtHeightResponse {
    pub power: Uint128,
    pub height: u64,
}

#[cw_serde]
pub struct TotalPowerAtHeightResponse {
    pub power: Uint128,
    pub height: u64,
}

#[cw_serde]
pub struct InfoResponse {
    pub info: ContractVersion,
}

#[cw_serde]
pub struct IsActiveResponse {
    pub active: bool,
}

mod tests {

    /// Make sure the enum has all of the fields we expect. This will
    /// fail to compile if not.
    #[test]
    fn test_macro_expansion() {
        use cosmwasm_schema::{cw_serde, QueryResponses};

        use cwd_macros::{active_query, token_query, voting_module_query};
        let query = Query::TokenContract {};

        #[token_query]
        #[voting_module_query]
        #[active_query]
        #[cw_serde]
        #[derive(QueryResponses)]
        enum Query {}

        match query {
            Query::TokenContract {} => (),
            Query::VotingPowerAtHeight { .. } => (),
            Query::TotalPowerAtHeight { .. } => (),
            Query::IsActive {} => (),
            Query::Info {} => (),
            Query::Dao {} => (),
        }
    }
}
