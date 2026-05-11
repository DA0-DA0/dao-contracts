# DAO ABC Factory

Factory contract that instantiates a [`cw-abc`](../cw-abc/) (Augmented Bonding Curve) for a DAO during DAO instantiation. Pairs with `dao-voting-token-staked` so the DAO's governance token is the curve's supply token.

## How it's wired

A typical DAO instantiation that uses an ABC for token launch:

1. The user instantiates `dao-dao-core` with a `voting_module` of type `dao-voting-token-staked` configured in `Existing` factory mode pointing at this `dao-abc-factory`.
2. `dao-dao-core` instantiates `dao-voting-token-staked`.
3. `dao-voting-token-staked` calls `ExecuteMsg::AbcFactory { code_id, instantiate_msg }` on this factory, where `instantiate_msg` is a [`cw_abc::msg::InstantiateMsg`](../cw-abc/src/msg.rs).
4. **The factory authenticates the caller** (see "Authentication" below).
5. The factory instantiates a `cw-abc` contract.
6. In the reply, the factory transfers ownership of the new ABC to the DAO core (pending) and returns a `TokenFactoryCallback` to the voting module containing the new token's denom and the issuer contract's address.
7. `dao-dao-core` runs the `ModuleInstantiateCallback` returned in the `TokenFactoryCallback`, which calls `AcceptOwnership` on the new ABC contract — completing the ownership handoff.

The DAO now owns the ABC. The supply token is registered as the DAO's voting token; staking that token gives voting power.

## Authentication

The factory has no `owner` and is freely callable by any address. To prevent an impostor "voting module" from spoofing the DAO relationship and having a freshly-minted ABC's ownership transferred to an attacker-chosen address, the factory uses a **reverse handshake** at the top of `execute_token_factory_factory`:

```rust
// 1. Ask the alleged voting module which DAO it belongs to.
let dao: Addr = querier.query_wasm_smart(&info.sender, &VotingModuleQueryMsg::Dao {})?;

// 2. Ask that DAO which contract it considers its voting module.
let claimed_voting_module: Addr =
    querier.query_wasm_smart(&dao, &DaoQueryMsg::VotingModule {})?;

// 3. Reject unless the round trip closes.
ensure!(
    claimed_voting_module == info.sender,
    ContractError::Unauthorized {}
);
```

This admits any contract that is genuinely a DAO's voting module (its DAO will name it as the voting module on its end), while rejecting impostor contracts that respond to `VotingModuleQueryMsg::Dao` with an attacker-chosen DAO address.

This pattern was added 2026-05-09 as part of the cw-abc audit revival (finding C-2). The same pattern is applied to `dao-test-custom-factory`'s analogous handlers.

## State

- `DAOS: Map<Addr, Empty>` — set of DAOs that have successfully spawned an ABC through this factory.
- `CURRENT_DAO: Item<Addr>` and `VOTING_MODULE: Item<Addr>` — TempState during the instantiate → reply round-trip; cleared at the end of the reply.

## Queries

- `Info {}` — cw2 contract version.
- `Daos { start_after, limit }` — paginated DAO list. Defaults to limit 25, descending order.

## Audit

A 2026-05-09 internal security review covered this contract together with `cw-abc` and `cw-curves`. See [`audits/2026-05-09-cw-abc-security-review.md`](../../../audits/2026-05-09-cw-abc-security-review.md). Ready for external audit pending CI test-tube run.

NOTE: this contract is NOT externally audited yet, use at your own risk.
