# Registering Protobuf Types

To use the `#proto` or `#stargate` transformation, you need to register the
protobuf messages that you want to use. This is done by providing a list of
`FileDescriptorSet`s that contain the protobuf messages and all their
dependencies.

```rust
use cw_jsonfilter::CwJsonFilter;
let cwjf = CwJsonFilter::new(vec![some_file_descriptor_set, another_file_descriptor_set]);
```

## Using Protobuf Types in Filters

In CosmWasm messages, protobuf types are represented as base64-encoded strings.

```rust
use serde_json::json;
use cw_jsonfilter::CwJsonFilter;

let cwjf = CwJsonFilter::new(vec![google_file_descriptor_set]);

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
