use cosmwasm_schema::{cw_serde, QueryResponses};

use dao_dao_macros::voting_module_query;

/// enum for testing. Important that this derives things / has other
/// attributes so we can be sure we aren't messing with other macros
/// with ours.
#[voting_module_query]
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
fn voting_module_query_derive() {
    let _test = Test::VotingPowerAtHeight {
        address: "foo".to_string(),
        height: Some(10),
    };

    let test = Test::TotalPowerAtHeight { height: Some(10) };

    // If this compiles we have won.
    match test {
        Test::Foo
        | Test::Bar(_)
        | Test::Baz { .. }
        | Test::TotalPowerAtHeight { height: _ }
        | Test::VotingPowerAtHeight {
            height: _,
            address: _,
        }
        | Test::Info {} => "yay",
        Test::Dao {} => "yay",
    };
}
