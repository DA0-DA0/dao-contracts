# CW Native Staked Balance Voting

[![docs.rs](https://img.shields.io/docsrs/dao-voting-native-staked)](https://docs.rs/dao-voting-native-staked/latest/dao_voting_native_staked/)

Simple native token voting contract which assumes the native denom
provided is not used for staking for securing the network e.g. IBC
denoms or secondary tokens (ION). Staked balances may be queried at an
arbitrary height. This contract implements the interface needed to be a DAO
DAO [voting
module](https://github.com/DA0-DA0/dao-contracts/wiki/DAO-DAO-Contracts-Design#the-voting-module).

If your chain uses Token Factory, consider using `dao-voting-token-factory-staked` for additional functionality including creating new tokens.
