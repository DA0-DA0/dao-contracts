# CW Native Staked Balance Voting

Simple native token voting contract which assumes the native denom
provided is not used for staking for securing the network e.g. IBC
denoms or secondary tokens (ION). Staked balances may be queried at an
arbitrary height. This contract implements the interface needed to be a DAO
DAO [voting
module](https://github.com/DA0-DA0/dao-contracts/wiki/DAO-DAO-Contracts-Design#the-voting-module).

