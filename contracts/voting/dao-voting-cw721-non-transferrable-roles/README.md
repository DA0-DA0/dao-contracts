# Stake CW721

This is a basic implementation of a cw721 staking contract. Staked
tokens can be unbonded with a configurable unbonding period. Staked
balances can be queried at any arbitrary height by external
contracts. This contract implements the interface needed to be a DAO
DAO [voting
module](https://github.com/DA0-DA0/dao-contracts/wiki/DAO-DAO-Contracts-Design#the-voting-module).
