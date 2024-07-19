# DAO Rewards Distributor

[![dao-rewards-distributor on crates.io](https://img.shields.io/crates/v/dao-rewards-distributor.svg?logo=rust)](https://crates.io/crates/dao-rewards-distributor)
[![docs.rs](https://img.shields.io/docsrs/dao-rewards-distributor?logo=docsdotrs)](https://docs.rs/dao-rewards-distributor/latest/cw20_stake_external_rewards/)

The `dao-rewards-distributor` works in conjuction with DAO voting modules to provide rewards over time for DAO members. The contract supports both cw20 and native Cosmos SDK tokens. The following voting power modules are supported for deriving staking reward allocations:

- `dao-voting-cw4`: for membership or group based DAOs
- `dao-voting-cw20-staked`: for cw20 token based DAOs.
- `dao-voting-cw721-staked`: for NFT based DAOs.
- `dao-voting-token-staked`: for native and Token Factory token based DAOs.

NOTE: this contract is NOT AUDITED and is _experimental_. USE AT YOUR OWN RISK.

## Instantiation and Setup

The contract is instantiated with a very minimal state.
An optional `owner` can be specified. If it is not, the owner is set
to be the address instantiating the contract.

### Hooks setup

After instantiating the contract it is VITAL to setup the required hooks for it to work. This is because to pay out rewards accurately, this contract needs to know about staking or voting power changes in the DAO.

This can be achieved using the `add_hook` method on contracts that support voting power changes, such as:

- `cw4-group`
- `dao-voting-cw721-staked`
- `dao-voting-token-staked`
- `cw20-stake`

### Registering a new reward denom

Only the `owner` can register new denoms for distribution.

Registering a denom for distribution expects the following config:

- `denom`, which can either be `Cw20` or `Native`
- `emission_rate`, which determines the `amount` of that denom to be distributed to all applicable addresses per `duration` of time. duration here may be declared in either time (seconds) or blocks. some example configurations may be:
  - `1000udenom` per 500 blocks
  - `1000udenom` per 24 hours
  - `0udenom` per any duration which effectively pauses the rewards
- `vp_contract` address, which will be used to determine the total and relative address voting power for allocating the rewards in a pro-rata basis
- `hook_caller` address, which will be authorized to call back into this contract with any voting power event changes. Example of such events may be:
  - user staking tokens
  - user unstaking tokens
  - user cw-721 state change event
  - cw-4 membership change event
- optional `withdraw_destination` address to be used in cases where after shutting down the denom reward distribution unallocated tokens would be sent to. One example use case of this may be some subDAO.

A denom being registered does not mean that any rewards will be distributed. Instead, it enables that to happen by enabling the registered reward denom to be funded.

Currently, a single denom can only have one active distribution configuration.

### Funding the denom to be distributed

Anyone can fund a denom to be distributed as long as that denom
is registered.

If a denom is not registered and someone attempts to fund it, an error will be thrown.

Otherwise, the funded denom state is updated in a few ways.

First, the funded period duration is calculated based on the amount of tokens sent and the configured emission rate. For instance, if 100_000udenom were funded, and the configured emission rate is 1_000udenom per 100 blocks, we derive that there are 100_000/1_000 = 100 epochs funded, each of which contain 100 blocks. We therefore funded 10_000 blocks of rewards.

Then the active epoch end date is re-evaluated, depending on its current value:

- If the active epoch never expires, meaning no rewards are being distributed, we take the funded period duration and add it to the current block.
- If the active epoch expires in the future, then we extend the current deadline with the funded period duration.
- If the active epoch had already expired, then we re-start the rewards distribution by adding the funded period duration to the current block.

### Updating denom reward emission rate

Only the `owner` can update the reward emission rate.

Updating the denom reward emission rate archives the active reward epoch and starts a new one.

First, the currently active epoch is evaluated. We find the amount of tokens that were earned to this point per unit of voting power and save that in the current epoch as its total earned rewards per unit of voting power.
We then bump the last update with that of the current block, and transition into the new epoch.

Active reward epoch is moved into the `historic_epochs`. This is a list of previously active reward emission schedules, along with their finalized amounts.

### Shutting down denom distribution

Only the `owner` can shutdown denom distribution.

Shutdown stops the denom from being distributed, calculates the amount of rewards that was allocated (and may or may not had been claimed yet), and claws that back to the `withdraw_address`.
