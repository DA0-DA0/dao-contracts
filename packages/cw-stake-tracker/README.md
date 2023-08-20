# Stake Tracker

This is a CosmWasm package for tracking the staked balance of a smart
contract.

The `StakeTracker` type here exposes a couple methods with the `on_`
prefix. These should be called whenever the contract performs an
action with x/staking. For example, when the contract delegates
tokens, it should call the `on_delegate` method to register that. Not
calling the method will cause the package to incorrectly track staked
values.

See
[`cw-vesting`](https://github.com/DA0-DA0/dao-contracts/blob/main/contracts/external/cw-vesting/SECURITY.md#slashing)
for an example of integrating this package into a smart contract and a
discussion of how to handle slash events.

