# `dao-voting-cw721-staked`

[![dao-voting-cw721-staked on crates.io](https://img.shields.io/crates/v/dao-voting-cw721-staked.svg?logo=rust)](https://crates.io/crates/dao-voting-cw721-staked)
[![docs.rs](https://img.shields.io/docsrs/dao-voting-cw721-staked?logo=docsdotrs)](https://docs.rs/dao-voting-cw721-staked/latest/dao_voting_cw721_staked/)

This is a basic implementation of an NFT staking contract.

Staked tokens can be unbonded with a configurable unbonding period. Staked balances can be queried at any arbitrary height by external contracts. This contract implements the interface needed to be a DAO DAO [voting module](https://github.com/DA0-DA0/dao-contracts/wiki/DAO-DAO-Contracts-Design#the-voting-module).

`dao-voting-cw721-staked` can be used with an `existing` NFT collection or to create a `new` `cw721` collection upon instantiation (with the DAO as admin and `minter`).

To support Stargaze NFTs and other custom NFT contracts or setups with minters (such as the Stargaze Open Edition minter), this contract also supports a `factory` pattern which takes a single `WasmMsg::Execute` message that calls into a custom factory contract.

**NOTE:** when using the factory pattern, it is important to only use a trusted factory contract, as all validation happens in the factory contract.

Those implementing custom factory contracts MUST handle any validation that is to happen, and the custom `WasmMsg::Execute` message MUST include `NftFactoryCallback` data respectively.

The [dao-test-custom-factory contract](../test/dao-test-custom-factory) provides an example of how this can be done and is used for tests. It is NOT production ready, but meant to serve as an example for building factory contracts.
