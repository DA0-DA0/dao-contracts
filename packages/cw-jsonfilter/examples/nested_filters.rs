use cw_jsonfilter::{CwJsonFilter, FilterResult};
use serde_json::{json, Value};

fn main() {
    // Filter can be as complex as you want
    let filter = json!({
        "$and": [
            {"$or": [
                {"age": {"$gte": 18}},
                {"is_student": true}
            ]},
            {"$not": {
                "$or": [
                    {"city": "New York"},
                    {"city": "Los Angeles"}
                ]
            }}
        ]
    });

    let obj1 = json!({"age": 25, "is_student": false, "city": "Chicago"});
    let obj2 = json!({"age": 16, "is_student": true, "city": "Miami"});
    let obj3 = json!({"age": 30, "is_student": false, "city": "New York"});

    println!("Filter:");
    println!("{filter}");
    println!("Objects:");
    println!("Object 1: {obj1}");
    println!("Object 2: {obj2}");
    println!("Object 3: {obj3}");

    // Matching objects against the filter
    match_objects(&filter, &obj1);
    match_objects(&filter, &obj2);
    match_objects(&filter, &obj3);
}

fn match_objects(filter: &Value, obj: &Value) {
    match CwJsonFilter::check(filter, obj) {
        FilterResult::Pass => println!("Filter matches the object"),
        FilterResult::Fail(err) => println!("Filter does not match the object: {err:?}"),
        FilterResult::Fatal(err) => println!("Fatal error: {err:?}"),
    }
}
