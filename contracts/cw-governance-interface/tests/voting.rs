use cw_governance_interface::cw_governance_voting_query;

/// enum for testing. Important that this derives things / has other
/// attributes so we can be sure we aren't messing with other macros
/// with ours.
#[cw_governance_voting_query]
#[derive(Clone)]
#[allow(dead_code)]
enum Test {
    Foo,
    Bar(u64),
    Baz { foo: u64 },
}

#[test]
fn it_works() {
    let _test = Test::VotingPowerAtHeight {
        address: "foo".to_string(),
        height: Some(10),
    };

    let test = Test::TotalPowerAtHeight { height: Some(10) };

    // If this compiles we have won.
    match test.clone() {
        Test::Foo
        | Test::Bar(_)
        | Test::Baz { .. }
        | Test::TotalPowerAtHeight { height: _ }
        | Test::VotingPowerAtHeight {
            height: _,
            address: _,
        } => "yay".to_string(),
    };
}
