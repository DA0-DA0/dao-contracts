use cw_jsonfilter::{CwJsonFilter, FilterResult, ProtobufDecoder};
use cw_protobuf_registry::protobuf::{base64_encode_protobuf, decode_protobuf};
use prost_reflect::{prost::Message, prost_types::FileDescriptorSet, DescriptorPool};
use serde_json::{json, Value};

fn main() {
    let crate_root = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let proto_path = std::path::Path::new(&crate_root).join("proto/string_bool_value.pb");

    let file_descriptor_set =
        FileDescriptorSet::decode(std::fs::read(proto_path).unwrap().as_slice()).unwrap();
    let pool = DescriptorPool::from_file_descriptor_set(file_descriptor_set.clone()).unwrap();

    struct MockProtobufDecoder {
        file_descriptor_set: FileDescriptorSet,
    }

    impl ProtobufDecoder for MockProtobufDecoder {
        fn decode(&self, message_name: String, value: Vec<u8>) -> Result<serde_json::Value, String> {
            decode_protobuf(self.file_descriptor_set.clone(), message_name, value)
        }
    }

    let cwjf = CwJsonFilter::new(Some(MockProtobufDecoder { file_descriptor_set }));

    // String filter

    let string_filter =
        json!({"someProto": {"#proto": {"type": "google.protobuf.StringValue", "value": "pass"}}});
    let base64_encoded_pass =
        base64_encode_protobuf(&pool, "google.protobuf.StringValue", &json!("pass"));
    let base64_encoded_not_pass =
        base64_encode_protobuf(&pool, "google.protobuf.StringValue", &json!("not_test"));

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
        base64_encode_protobuf(&pool, "google.protobuf.BoolValue", &json!(true));
    let base64_encoded_false =
        base64_encode_protobuf(&pool, "google.protobuf.BoolValue", &json!(false));
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

fn match_objects<D: ProtobufDecoder>(cwjf: &CwJsonFilter<D>, filter: &Value, obj: &Value) {
    match cwjf.matches(filter, obj) {
        FilterResult::Pass => println!("Filter matches the object"),
        FilterResult::Fail(err) => println!("Filter does not match the object: {:?}", err),
        FilterResult::Fatal(err) => println!("Fatal error: {:?}", err),
    }
}
