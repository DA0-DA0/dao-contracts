use cosmwasm_schema::{cw_serde, QueryResponses};
use dao_macros::proposal_module_query;

#[proposal_module_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum Query {}

mod tests {
    /// Make sure the enum has all of the fields we expect. This will
    /// fail to compile if not.
    #[test]
    fn test_macro_expansion() {
        use super::Query;

        let query = Query::Info {};

        match query {
            Query::Dao {} => (),
            Query::Info {} => (),
            Query::NextProposalId {} => (),
        }
    }
}
