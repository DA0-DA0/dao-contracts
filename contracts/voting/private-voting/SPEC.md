# Private voting spec

This document assumes you have read the README.

The ballot struct is of the form

```rust
pub struct Ballot<H: HashDigest> {
    pub ballot_id: Uint64,
    pub voting_set_id: Uint64,
    pub hashed_nullfier_deriviation_key: H,
}
```

When a ballot is made, it is added to an `active_ballot_merkle_tree`. (TODO: when optimizing proof times and client UX, revisit alternate VC's. Double-batched-accumulators seem better on all fronts)

We also maintain a struct for each user:

```rust
pub struct User<H: HashDigest> {
    pub address: cw_std::Addr,
    pub ballot_ids: Vec<Uint64>,
    // notepad that can be used to restore access to nullifier_deriviation_key,
    // in event of client side data loss.
    // Needed if we assume wallets won't update. If wallets update, then 
    // nullifier_deriviation_key can be HD-derived.
    pub encrypted_notepad: Vec[u8],
}
```

## Nullifier

We create nullifiers for every vote, by making a circuit for:

* `Open pk where H(pk) = hashed_auth_key`
* `VRF(pk, ballot_id, vote_id) = nullifier`

TODO: See if Anoma has a circuit we can re-use

## Circuits
