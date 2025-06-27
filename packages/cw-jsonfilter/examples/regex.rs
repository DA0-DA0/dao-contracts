use cw_jsonfilter::{CwJsonFilter, FilterResult};
use serde_json::{json, Value};

fn main() {
    // You can filter text with regex
    let filter = json!({"description": {"$regex": "(Rust|Python)"}});

    let obj1 = json!({"name": "John Doe", "description": "Enthusiastic about programming in Rust"});
    let obj2 = json!({"name": "Alice Smith", "description": "Experienced in web development"});
    let obj3 = json!({"name": "Bob Brown", "description": "Loves to code in Python"});

    println!("Filter:");
    println!("{}", filter);
    println!("Objects:");
    println!("Object 1: {}", obj1);
    println!("Object 2: {}", obj2);
    println!("Object 3: {}", obj3);

    match_objects(&filter, &obj1);
    match_objects(&filter, &obj2);
    match_objects(&filter, &obj3);
}

fn match_objects(filter: &Value, obj: &Value) {
    match CwJsonFilter::check(filter, obj) {
        FilterResult::Pass => println!("Filter matches the object"),
        FilterResult::Fail(err) => println!("Filter does not match the object: {:?}", err),
        FilterResult::Fatal(err) => println!("Fatal error: {:?}", err),
    }
}
