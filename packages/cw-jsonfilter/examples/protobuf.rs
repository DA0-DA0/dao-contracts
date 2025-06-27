use base64::Engine;
use cw_jsonfilter::{CwJsonFilter, FilterResult, BASE64_ENGINE};
use prost::Message;
use prost_reflect::{prost_types::FileDescriptorSet, DescriptorPool, DynamicMessage};
use serde_json::{json, Deserializer, Value};

fn main() {
    let crate_root = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let proto_path = std::path::Path::new(&crate_root).join("proto/string_bool_value.pb");

    let file_descriptor_sets =
        vec![FileDescriptorSet::decode(std::fs::read(proto_path).unwrap().as_slice()).unwrap()];

    let cwjf = CwJsonFilter::new(file_descriptor_sets);
    let pool = cwjf.pool.clone().unwrap();

    // String filter

    let string_filter =
        json!({"someProto": {"#proto": {"type": "google.protobuf.StringValue", "value": "pass"}}});
    let base64_encoded_pass =
        encode_protobuf_base64(&pool, "google.protobuf.StringValue", &json!("pass"));
    let base64_encoded_not_pass =
        encode_protobuf_base64(&pool, "google.protobuf.StringValue", &json!("not_test"));

    let obj1 = json!({"someProto": base64_encoded_pass});
    let obj2 = json!({"someProto": base64_encoded_not_pass});

    println!();
    println!("String filter:");
    println!("{}", string_filter);
    println!("String objects:");
    println!("Object 1: {}", obj1);
    match_objects(&cwjf, &string_filter, &obj1);
    println!("Object 2: {}", obj2);
    match_objects(&cwjf, &string_filter, &obj2);
    println!();

    // Bool filter

    let bool_filter =
        json!({"someProto": {"#proto": {"type": "google.protobuf.BoolValue", "value": true}}});

    let base64_encoded_true =
        encode_protobuf_base64(&pool, "google.protobuf.BoolValue", &json!(true));
    let base64_encoded_false =
        encode_protobuf_base64(&pool, "google.protobuf.BoolValue", &json!(false));
    let obj1 = json!({"someProto": base64_encoded_true});
    let obj2 = json!({"someProto": base64_encoded_false});

    println!("Bool filter:");
    println!("{}", bool_filter);
    println!("Bool objects:");
    println!("Object 1: {}", obj1);
    match_objects(&cwjf, &bool_filter, &obj1);
    println!("Object 2: {}", obj2);
    match_objects(&cwjf, &bool_filter, &obj2);
    println!();
}

fn match_objects(cwjf: &CwJsonFilter, filter: &Value, obj: &Value) {
    match cwjf.matches(filter, obj) {
        FilterResult::Pass => println!("Filter matches the object"),
        FilterResult::Fail(err) => println!("Filter does not match the object: {:?}", err),
        FilterResult::Fatal(err) => println!("Fatal error: {:?}", err),
    }
}

fn encode_protobuf_base64(pool: &DescriptorPool, message_name: &str, value: &Value) -> String {
    let value_str = value.to_string();
    let message_descriptor = pool.get_message_by_name(message_name).unwrap();

    let mut deserializer = Deserializer::from_str(&value_str);
    let dynamic_message =
        DynamicMessage::deserialize(message_descriptor, &mut deserializer).unwrap();
    deserializer.end().unwrap();

    // Encode the message data to bytes and then base64 encode it.
    BASE64_ENGINE.encode(dynamic_message.encode_to_vec())
}
