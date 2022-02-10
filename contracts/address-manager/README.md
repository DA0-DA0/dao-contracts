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
first.

Placing many, countless addresses into this contract may cause it to
become unusable as it will run into out of gas issues. The contract
will only become locked at the point when the number of addresses
becomes so enormous that a single `log(N)` operation over a B-Tree set
excedes the gas limit.
