use std::collections::HashSet;

use base64::Engine;
use prost_reflect::{prost::Message, DescriptorPool, DynamicMessage};
use serde_json::Deserializer;

use crate::BASE64_ENGINE;

/// Get the protobuf messages referenced by a filter.
pub fn get_protobuf_messages(filter: &serde_json::Value) -> HashSet<String> {
    let mut messages = HashSet::new();
    inner_get_protobuf_messages(filter, &mut messages);
    messages
}

/// Get the protobuf messages referenced by a filter, recursively.
fn inner_get_protobuf_messages(filter: &serde_json::Value, messages: &mut HashSet<String>) {
    if let serde_json::Value::Object(filter_map) = filter {
        for (key, value) in filter_map {
            // If the key is the #proto transformer, add the key to the set.
            if key == "#proto" {
                if let Some(serde_json::Value::String(proto_type)) = value
                    .as_object()
                    .and_then(|proto_arg| proto_arg.get("type"))
                {
                    messages.insert(proto_type.clone());
                }
            }

            // Recurse on the value.
            inner_get_protobuf_messages(value, messages);
        }
    }
}

/// Encode a protobuf message to base64.
///
/// # Arguments
///
/// * `pool` - The descriptor pool to use to get the message descriptor.
/// * `message_name` - The name of the message to encode.
/// * `value` - The value to encode.
///
/// # Returns
///
/// The base64 encoded protobuf message value.
pub fn base64_encode_protobuf(
    pool: &DescriptorPool,
    message_name: impl Into<String>,
    value: &serde_json::Value,
) -> String {
    let value_str = value.to_string();
    let message_descriptor = pool.get_message_by_name(&message_name.into()).unwrap();

    let mut deserializer = Deserializer::from_str(&value_str);
    let dynamic_message =
        DynamicMessage::deserialize(message_descriptor, &mut deserializer).unwrap();
    deserializer.end().unwrap();

    // Encode the message data to bytes and then base64 encode it.
    BASE64_ENGINE.encode(dynamic_message.encode_to_vec())
}
