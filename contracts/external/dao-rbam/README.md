# dao-rbam

[![dao-rbam on crates.io](https://img.shields.io/crates/v/dao-rbam.svg?logo=rust)](https://crates.io/crates/dao-rbam)
[![docs.rs](https://img.shields.io/docsrs/dao-rbam?logo=docsdotrs)](https://docs.rs/dao-rbam/latest/dao_rbam/)

Roles-based Authorization Module (RBAM) that lets DAOs configure
organization-wide roles and authorization policies.

## Filtering

Authorizations for a role are defined by a
[`cw-jsonfilter`](../../../packages/cw-jsonfilter/) filter. The JSON filter
format, operators, and transformations are described in the [filter
documentation](../../../packages/cw-jsonfilter/docs/filter.md).

You can make very complex filters by combining the various operators and
transformations.

For example:

```json
{
  "bank": {
    "send": {
      "to_address": "cosmos1...",
      "amount": [
        {
          "denom": "uatom",
          "amount": { "#to_number": { "$between": [1000, 2000] } }
        }
      ]
    }
  }
}
```

This filter would allow the role-holder to send between 1,000 and 2,000 uatom to
the `cosmos1...` address.

Note the `#to_number` transformation, which converts the string `"1000"` to the
number `1000` before the comparison operator is applied, since bank messages use
strings for amounts. Since strings can be compared lexicographically, strings
and numbers are both valid inputs to comparison operators, and thus strings are
not coerced to numbers automatically and require manual conversion like this.

```json
{
  "bank": {
    "send": {
      "to_address": {
        "$in": ["cosmos123...", "cosmos145..."]
      },
      "amount": [
        { "denom": "uatom", "amount": { "#to_number": { "$lte": 10000 } } }
      ]
    }
  }
}
```

This filter would allow the role-holder to send <= 10,000 uatom to either
`cosmos123...` or `cosmos145...`.

### Base64

CosmWasm messages encode binary data as base64-encoded strings, which can be
filtered with the `#base64` transformation. To authorize contract instantiations
or executions, for example, you first decode the base64-encoded message into a
JSON object, and then filter on the JSON object, like so:

```json
{
  "wasm": {
    "execute": {
      "contract_addr": "exact_contract",
      "msg": {
        "#base64": {
          "update_config": {
            "config": {
              "name": { "$contains": "new_name" }
            }
          }
        }
      },
      "funds": []
    }
  }
}
```

This filter would allow the role-holder to execute the `exact_contract` contract
with the `update_config` message, ONLY if the `name` field contains `new_name`.
Other fields can be anything, since no filter is specified for them. This also
ensures that no funds are sent with the message, since the `funds` field must be
an empty array.

### Stargate/Protobuf

You can use the `#stargate` transformation to filter on custom protobuf
messages. See the [protobuf
documentation](../../../packages/cw-jsonfilter/docs/protobuf.md) for more
details.

To enable protobuf decoding support, you must first register the protobuf types
with the contract, and then create authorizations that use the transformation.

1. Register the protobuf types with the contract by executing the
   `RegisterProtobufs` message. The `file_descriptor_sets` field is a list of
   file descriptor sets, each of which is encoded as bytes. The protobuf
   documentation linked above describes how to generate the file descriptor
   sets. The generated `.pb` file is the file descriptor set encoded as bytes.

   ```json
   {
     "register_protobufs": {
       "file_descriptor_sets": [
         [1, 2, 3] // file descriptor set 1
         [4, 5, 6] // file descriptor set 2
         [7, 8, 9] // file descriptor set 3
       ]
     }
   }
   ```

2. Create an authorization that uses the `#stargate` transformation. The
   `type_url` field is the full type URL of the protobuf message (prefixed with
   a `/` like normal), and the `value` field is the value to filter on. This
   decodes the base64-encoded protobuf message into a JSON object that can be
   filtered on.

   ```json
   {
     "create_authorization": {
       "role_id": 1,
       "name": "My Authorization",
       "filter": {
         "#stargate": {
           "type_url": "/some.custom.protobuf.message",
           "value": {
             "field1": "value1",
             "field2": "value2",
             "field3": {
               "$between": [0, 100]
             },
             "field4": {
               "#to_number": {
                 "$gte": 50
               }
             }
           }
         }
       }
     }
   }
   ```
