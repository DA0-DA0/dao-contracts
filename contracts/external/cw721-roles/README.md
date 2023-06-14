# cw721-roles

This is a non-transferable NFT contract intended for use with DAOs. `cw721-roles` has an extension that allows for each NFT to have a `weight` associated with it, and also implements much of the functionality behind the [cw4-group contract](https://github.com/CosmWasm/cw-plus/tree/main/contracts/cw4-group) (credit to [Confio](https://confio.gmbh/) for their work on that).

All methods of this contract are only callable via the configurable `minter` when the contract is created. It is primarily intended for use with DAOs.

The `mint`, `burn`, `send`, and `transfer` methods have all been overriden from their default `cw721-base` versions, but work roughly the same with the caveat being they are only callable via the `minter`. All methods related to approvals are unsupported.

## Extensions

`cw721-roles` contains the following extensions:

Token metadata has been extended with a weight and an optional human readable on-chain role which may be used in separate contracts for enforcing additional permissions.

```rust
pub struct MetadataExt {
    /// Optional on-chain role for this member, can be used by other contracts to enforce permissions
    pub role: Option<String>,
    /// The voting weight of this role
    pub weight: u64,
}
```

The contract has an additional execution extension that includes the ability to add and remove hooks for membership change events, as well as update a particular token's `token_uri`, `weight`, and `role`. All of these are only callable by the configured `minter`.

```rust
pub enum ExecuteExt {
    /// Add a new hook to be informed of all membership changes.
    /// Must be called by Admin
    AddHook { addr: String },
    /// Remove a hook. Must be called by Admin
    RemoveHook { addr: String },
    /// Update the token_uri for a particular NFT. Must be called by minter / admin
    UpdateTokenUri {
        token_id: String,
        token_uri: Option<String>,
    },
    /// Updates the voting weight of a token. Must be called by minter / admin
    UpdateTokenWeight { token_id: String, weight: u64 },
    /// Udates the role of a token. Must be called by minter / admin
    UpdateTokenRole {
        token_id: String,
        role: Option<String>,
    },
}
```

The query extension implements queries that are compatible with the previously mentioned [cw4-group contract](https://github.com/CosmWasm/cw-plus/tree/main/contracts/cw4-group).

```ignore
pub enum QueryExt {
    /// Total weight at a given height
    #[returns(cw4::TotalWeightResponse)]
    TotalWeight { at_height: Option<u64> },
    /// Returns the weight of a certain member
    #[returns(cw4::MemberResponse)]
    Member {
        addr: String,
        at_height: Option<u64>,
    },
    /// Shows all registered hooks.
    #[returns(cw_controllers::HooksResponse)]
    Hooks {},
}
```
