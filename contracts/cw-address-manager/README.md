# Address Manager

Small contract for managing a sorted list of addresses. Might be used
for keeping a list of favorite contracts.

Addresses are added in the form:

```rust
struct AddressItem {
	pub addr: Addr,
	pub priority: u32,
}
```

One may then query for the contract list and addresses will be
returned in sorted order with the highest priority ones returned
first. One may also ask the contract if a particular address is in the
list. See `src/msg.rs` for supported messages and documentation.
