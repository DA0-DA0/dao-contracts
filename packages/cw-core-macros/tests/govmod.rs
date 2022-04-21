use cw_core_macros::govmod_query;

#[govmod_query]
#[derive(Clone)]
#[allow(dead_code)]
enum Test {
    Foo,
    Bar(u64),
    Baz { foo: u64 },
}

#[test]
fn govmod_query_derive() {
    let test = Test::Info {};

    // If this compiles we have won.
    match test {
        Test::Foo | Test::Bar(_) | Test::Baz { .. } | Test::Info {} => "yay",
    };
}
