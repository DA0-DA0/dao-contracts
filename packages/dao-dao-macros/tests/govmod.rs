use cosmwasm_schema::{cw_serde, QueryResponses};

use dao_dao_macros::proposal_module_query;

#[proposal_module_query]
#[allow(dead_code)]
#[cw_serde]
#[derive(QueryResponses)]
enum Test {
    #[returns(String)]
    Foo,
    #[returns(String)]
    Bar(u64),
    #[returns(String)]
    Baz { waldo: u64 },
}

#[test]
fn proposal_module_query_derive() {
    let test = Test::Dao {};

    // If this compiles we have won.
    match test {
        Test::Foo | Test::Bar(_) | Test::Baz { .. } | Test::Dao {} => "yay",
        Test::Info {} => "yay",
        Test::NextProposalId {} => "yay",
    };
}
