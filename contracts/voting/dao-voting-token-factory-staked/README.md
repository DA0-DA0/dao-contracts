# Token Factory Staked Balance Voting

Simple native or token factory based voting contract which assumes the native denom
provided is not used for staking for securing the network e.g. IBC
denoms or secondary tokens (ION). Staked balances may be queried at an
arbitrary height. This contract implements the interface needed to be a DAO
DAO [voting
module](https://github.com/DA0-DA0/dao-contracts/wiki/DAO-DAO-Contracts-Design#the-voting-module).

This contract requires having the Token Factory module on your chain, which allows the creation of new native tokens. If your chain does not have this module, use `dao-voting-native-staked` instead.

