# dao-voting-incentives

> **WARNING:** THIS CONTRACT IS NOT AUDITED AND IS EXPERIMENTAL. USE AT YOUR OWN RISK.

The `dao-voting-incentives` contract is designed to boost participation in DAO governance by offering rewards for voting on proposals. This innovative mechanism encourages active community involvement and more representative decision-making within DAOs.

## Features

- Flexible reward distribution for both native and CW20 tokens
- Time-bound incentive periods
- Fair reward calculation based on voting activity

## Instantiation

To deploy the contract, provide the following parameters:

```rust
pub struct InstantiateMsg {
    pub owner: String,
    pub denom: UncheckedDenom,
    pub expiration: Expiration,
}
```

- `owner`: The DAO address that will manage the contract and receive vote hooks
- `denom`: The token denomination (native or CW20) to be distributed as rewards
- `expiration`: The end date of the voting incentives period

## Setup

1. Deploy the `dao-voting-incentives` contract
2. Add the contract's address as a `VoteHook` to your DAO's proposal module (`dao-proposal-single` or `dao-proposal-multiple`)
3. Ensure the DAO is set as the `owner` of the contract
4. Fund the contract with the specified reward tokens

## Key Functions

### Execute Messages

- `VoteHook(VoteHookMsg)`: Tracks voting activity (automatically called by the proposal module)
- `Claim {}`: Allows voters to claim their earned rewards after the incentive period ends
- `Expire {}`: Finalizes the incentive period, enabling reward claims
- `UpdateOwnership(Action)`: Manages contract ownership
- `Receive(Cw20ReceiveMsg)`: Handles incoming CW20 tokens for rewards

### Query Messages

- `Config {}`: Retrieves the contract's configuration
- `Rewards { address: String }`: Gets the claimable rewards for a specific address
- `Votes { address: String }`: Returns the number of votes cast by an address

## Reward Calculation

Rewards are calculated using the following formula:

```
reward(user) = votes(user) * contract_balance / total_votes
```

This ensures a fair distribution based on each user's voting activity relative to the total participation.

## Important Notes

- If no votes are cast during the incentive period, all funds are returned to the owner (DAO) upon expiration
- Rewards can only be claimed after the incentive period has ended
