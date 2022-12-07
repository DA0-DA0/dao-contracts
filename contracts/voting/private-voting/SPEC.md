# Private voting spec

This document assumes you have read the README.

## Concepts

### Ballots

Every ballot has an associated ballot ID. Each ballot represents a fixed amount of weight.

The ballot struct is of the form

```rust
pub struct Ballot<H: HashDigest> {
    pub ballot_id: Uint64,
    pub voting_set_id: Uint64,
    pub hashed_nullfier_deriviation_key: H,
}
```

### Ballot trees

For governance, a ballot can become invalid for later proposals, due to a voter's voting power decreasing.
This is a complicating factor over Zcash. In Zcash a UTXO from any point in the past (called a `note`), is valid as long as it has not been spent yet.
So when creating a `note`, you add it to an append-only merkle tree called the `note commitment tree`. Every note commits to a hidden value called a `nullifier`, and to spend a note you publicly reveal the nullifier. You then check if a spend is valid, by ensuring the note being spent is committed to in the commitment tree, and if its nullifier has not been spent yet.
This on its own insufficient for us, as we require a second mechanism to invalidate ballots, that cannot depend on input from the ballot owner.
However, the `note commitment tree` being append-only is of vital importance, this is what enables a user to update the merkle path to their note commitment locally, with very few queries to a server (and no de-anonymization).

To imitate this, we create an append only `ballot_merkle_tree`, and a sorted-by-key `invalidated_ballot_merkle_tree`. (TODO: Consider using more proof friendly structures than merkle trees)

When a ballot is made, it is added to an `ballot_merkle_tree`.

When a ballot is invalidated, it is added to the `invalidated_ballot_merkle_tree`.
(TODO: Leakage due to querying this, and using H(ballot id))

### User struct

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

### Nullifiers

We create nullifiers for every vote, by making a circuit for:

* `Open pk where H(pk) = hashed_auth_key`
* `VRF(pk, ballot_id, proposal_id) = nullifier`

TODO: See if Anoma has a circuit we can re-us

## Circuits

* Nullifier derivation circuit

### Optimizing for circuits

* ballot merkle tree -> can just not pay for hashing top levels, by unrolling into public input.
* invalidated ballot merkle tree -> any accumulator that supports non-inclusion proofs. Perhaps KZG.
