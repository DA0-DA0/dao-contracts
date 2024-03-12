# DAO Rewards Distributor

[![dao-rewards-distributor on crates.io](https://img.shields.io/crates/v/dao-rewards-distributor.svg?logo=rust)](https://crates.io/crates/dao-rewards-distributor)
[![docs.rs](https://img.shields.io/docsrs/dao-rewards-distributor?logo=docsdotrs)](https://docs.rs/dao-rewards-distributor/latest/cw20_stake_external_rewards/)

The `dao-rewards-distributor` works in conjuction with DAO voting modules to provide rewards over time for DAO members. The contract supports both cw20 and native Cosmos SDK tokens. The following voting power modules are supported:
- `dao-voting-cw4`: for membership or group based DAOs
- `dao-voting-cw20-staked`: for cw20 token based DAOs.
- `dao-voting-cw721-staked`: for NFT based DAOs.
- `dao-voting-token-staked`: for native and Token Factory token based DAOs.

NOTE: this contract is NOT AUDITED and is _experimental_. USE AT YOUR OWN RISK.

## Instantiation and Setup

The contract is instantiated with a number of parameters:
- `owner`: The owner of the contract. Is able to fund the contract and update the reward duration.
- `vp_contract`:  A DAO DAO voting power module contract address, used to determine membership in the DAO over time.
- `hook_caller`: An optional contract that is allowed to call voting power change hooks.  Often, as in `dao-voting-token-staked` and `dao-voting-cw721-staked` the vp_contract calls hooks for power change events, but sometimes they are separate. For example, the `cw4-group` contract is separate from the `dao-voting-cw4` contract and since the `cw4-group` contract fires the membership change events, it's address would be used as the `hook_caller`.
- `reward_denom`: the denomination of the reward token, can be either a cw20 or native token.
- `reward_duration`: the time period over which rewards are to be paid out in blocks.

After instantiating the contract it is VITAL to setup the required hooks for it to work. This is because to pay out rewards accurately, this contract needs to know about staking or voting power changes in the DAO.

This can be achieved using the `add_hook` method on contracts that support voting power changes, which are:
- `cw4-group`
- `dao-voting-cw721-staked`
- `dao-voting-token-staked`
- `cw20-stake`

Finally, the contract needs to be funded with a token matching the denom specified in the `reward_denom` field during instantiation. This can be achieved by calling the `fund` method on the `dao-rewards-distributor` smart contract, and sending along the appropriate funds.
