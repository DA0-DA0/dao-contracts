use cw_jsonfilter::{test, FilterResult};
use serde_json::json;

fn main() {
    let filter = json!({"name": "John", "age": 30});
    let obj = json!({"name": "John", "age": 30, "city": "New York"});

    println!("Applying filter:");
    println!("{}", filter);
    println!("To object:");
    println!("{}", obj);

    match test(&filter, &obj) {
        FilterResult::Pass => println!("Filter matches the object"),
        FilterResult::Fail(err) => println!("Filter does not match the object: {:?}", err),
        FilterResult::Fatal(err) => println!("Fatal error: {:?}", err),
    }
}
