# dao-proposal-incentives

> **WARNING:** THIS CONTRACT IS NOT AUDITED AND IS EXPERIMENTAL. USE AT YOUR OWN RISK.

## Overview

The `dao-proposal-incentives` contract empowers DAOs to boost member engagement by automatically rewarding successful proposals. This approach encourages active participation and high-quality contributions to DAO governance.

### Key Features

- Automatic rewards for passed proposals
- Support for both native and CW20 tokens
- Dynamic incentive adjustment
- Seamless integration with existing DAO modules

## How It Works

1. **Setup**: The DAO instantiates the contract and sets initial reward parameters.
2. **Funding**: The contract is funded with tokens for rewards.
2. **Integration**: The contract is added as a proposal hook to the DAO's voting module.
3. **Proposal Lifecycle**: When a proposal passes, the contract automatically rewards the proposer.
4. **Flexible Management**: The DAO can adjust reward amounts and token types as needed.

## Usage Guide

### Instantiation

To set up the contract, provide:

- `owner`: The DAO's address (for sending proposal hooks)
- `proposal_incentives`: Reward configuration using `ProposalIncentivesUnchecked`

Example:
```rust
let msg = InstantiateMsg {
    owner: "dao_address".to_string(),
    proposal_incentives: ProposalIncentivesUnchecked {
        rewards_per_proposal: Uint128::new(1000),
        denom: UncheckedDenom::Native("ujuno".to_string()),
    },
};
```

### Configuration

1. Add this contract as a `ProposalHook` to your DAO's voting module (`dao-voting-single` or `dao-voting-multiple`).
2. Ensure the DAO is set as the contract `owner` for proper management.

### Key Functions

#### Execute Messages

1. **ProposalHook(ProposalHookMsg)**: Handles proposal status changes and reward distribution.
2. **UpdateOwnership(cw_ownable::Action)**: Manages contract ownership.
3. **UpdateProposalIncentives**: Allows the DAO to modify reward settings.
4. **Receive(Cw20ReceiveMsg)**: Processes incoming CW20 tokens for rewards.

#### Query Messages

- **ProposalIncentives { height: Option<u64> }**: Retrieves current or historical incentive configurations.
