use cwd_macros::proposal_module_query;

#[proposal_module_query]
#[derive(Clone)]
#[allow(dead_code)]
enum Test {
    Foo,
    Bar(u64),
    Baz { foo: u64 },
}

#[test]
fn proposal_module_query_derive() {
    let test = Test::Dao {};

    // If this compiles we have won.
    match test {
        Test::Foo | Test::Bar(_) | Test::Baz { .. } | Test::Dao {} => "yay",
    };
}
