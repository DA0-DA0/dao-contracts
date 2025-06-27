# Registering Protobuf Types

To use the `#proto` operator, you need to register the protobuf types you want
to use. This is done by providing a list of `FileDescriptorSet`s that contain
the protobuf types and all their dependencies.

```rust
let cwjf = CwJsonFilter::new(vec![some_file_descriptor_set, another_file_descriptor_set]);
```

## Generating FileDescriptorSets

A `FileDescriptorSet` can be generated using the `buf` toolchain CLI, if your
project uses it:

```bash
buf build \
  --exclude-source-info \
  --exclude-source-retention-options \
  --as-file-descriptor-set \
  --output fds.pb
# add --type=<TYPE> to generate for a specific type and all its dependencies
```

or using the basic `protoc` CLI:

```bash
protoc --descriptor_set_out=store_code.pb \
       --include_imports \
       fds.proto
```
