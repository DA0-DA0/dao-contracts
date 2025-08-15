# Protobuf Types in Filters

To use the `#proto` or `#stargate` transformation, you need to register a
protobuf decoder function that can decode a protobuf message into a JSON value
with only its name and the protobuf encoded bytes.

```rust
use serde_json::json;
use cw_jsonfilter::{CwJsonFilter, ProtobufDecoder};
use prost_reflect::{prost::Message, prost_types::FileDescriptorSet, DescriptorPool, DynamicMessage};

// decoder type that will implement the ProtobufDecoder trait
struct MyDecoder {
    pool: DescriptorPool,
}

impl ProtobufDecoder for MyDecoder {
    fn decode(&self, message_name: String, value: Vec<u8>) -> Result<serde_json::Value, String> {
        let message_descriptor =
          self.pool.get_message_by_name(&message_name)
            .ok_or_else(|| {
              format!(
                  "message descriptor not found in pool for `{}`",
                  message_name
              )
            })?;

        let dynamic_message =
          DynamicMessage::decode(message_descriptor, value.as_slice())
            .map_err(|e| format!("failed to decode protobuf value: {}", e))?;

        let json = serde_json::to_value(dynamic_message)
            .map_err(|e| format!("failed to serialize decoded protobuf value as JSON: {}", e))?;

        Ok(json)
    }
}

let proto_fds_path = "path/to/proto/file.pb";
let file_descriptor_set =
  FileDescriptorSet::decode(std::fs::read(proto_fds_path).unwrap().as_slice()).unwrap();
let pool = DescriptorPool::from_file_descriptor_set(file_descriptor_set).unwrap();
let decoder = MyDecoder { pool };
let cwjf = CwJsonFilter::new(Some(decoder));

// To check if a JSON object matches a filter:
let filter = json!({"proto_data": { "#proto": { "type": "google.protobuf.StringValue", "value": "Hello, world!" } }});
let obj = json!({"proto_data": "<BASE64_ENCODED_PROTOBUF_DATA>" }});

cwjf.matches(&filter, &obj);
```

And they appear as `Stargate` messages:

```rust
use serde_json::json;
use cosmwasm_std::{CosmosMsg, coins};

// With the `#proto` transformation:
let filter = json!({
  "stargate": {
    "type_url": "/cosmos.bank.v1beta1.MsgSend",
    "value": {
      "#proto": {
        "type": "cosmos.bank.v1beta1.MsgSend",
        "value": {
          "from_address": "cosmos1...",
          "to_address": "cosmos1...",
          "amount": [{
            "denom": "uatom",
            "amount": {
              "$between": [0, 10_000_000]
            }
          }]
        }
      }
    }
  }
});

// Shorthand with the `#stargate` transformation:
let filter = json!({
  "#stargate": {
    "type_url": "/cosmos.bank.v1beta1.MsgSend",
    "value": {
      "from_address": "cosmos1...",
      "to_address": "cosmos1...",
      "amount": [{
        "denom": "uatom",
        "amount": {
          "$between": [0, 10_000_000]
        }
      }]
    }
  }
});

// Message that will pass the filter using `osmosis-std`'s protobuf types:
let msg = CosmosMsg::Stargate {
  type_url: "/cosmos.bank.v1beta1.MsgSend".to_string(),
  value: osmosis_std::types::cosmos::bank::v1beta1::MsgSend {
    from_address: "cosmos1...".to_string(),
    to_address: "cosmos1...".to_string(),
    amount: vec![osmosis_std::types::cosmos::base::v1beta1::Coin {
        denom: "uatom".to_string(),
        amount: "5000000".to_string(),
    }],
  }.into(),
};
```

## Generating FileDescriptorSets

A `FileDescriptorSet` can be generated using the `buf` toolchain CLI, if your
project uses it:

```bash
buf build \
  --exclude-source-info \
  --exclude-source-retention-options \
  --as-file-descriptor-set \
  # generate for a specific type and all its dependencies
  --type=<TYPE> \
  --output fds.pb
```

or using the basic `protoc` CLI:

```bash
protoc --descriptor_set_out=fd.pb \
       --include_imports \
       <PROTO_FILE>.proto
```
