# dao-proposal-incentives

[![dao-proposal-incentives on crates.io](https://img.shields.io/crates/v/dao-proposal-incentives.svg?logo=rust)](https://crates.io/crates/dao-proposal-incentives)
[![docs.rs](https://img.shields.io/docsrs/dao-proposal-incentives?logo=docsdotrs)](https://docs.rs/dao-proposal-incentives/latest/cw_admin_factory/)

This contract enables DAO's to incentivize members for making successful proposals. By integrating this contract, DAO's can automatically reward members whose proposals are successfully passed, using either native tokens or CW20 tokens.

## Instantiate 

To instantiate the contract, provide the following parameters:

- `owner`: The DAO sending this contract proposal hooks.
- `proposal_incentives`: Configuration for the incentives to be awarded for successful proposals. This should be specified using the `ProposalIncentivesUnchecked` structure.

## Setup

- This contract should be added as a `ProposalHook` to either the `dao-voting-single` or `dao-voting-multiple` proposal modules.
- The DAO must be set as the `owner` of this contract to manage incentives and ownership.

## Execute

- **ProposalHook(ProposalHookMsg)**: Triggered when a proposal's status changes. This is used to evaluate and potentially reward successful proposals.
- **UpdateOwnership(cw_ownable::Action)**: Updates the ownership of the contract. This can be used to transfer ownership or perform other ownership-related actions.
- **UpdateProposalIncentives { proposal_incentives: ProposalIncentivesUnchecked }**: Updates the incentives configuration. This allows the DAO to modify the rewards for successful proposals.
- **Receive(Cw20ReceiveMsg)**: Handles the receipt of CW20 tokens. This is necessary for managing CW20-based incentives.

## Query

- **ProposalIncentives { height: Option<u64> }**: Returns the current configuration of the proposal incentives. The `height` parameter is optional and can be used to query the incentives at a specific blockchain height, providing a snapshot of the incentives at that point in time.

## Configuration

The incentives can be adjusted at any time by the owner of the contract. The rewards are determined based on the configuration at the proposal's `start_time`. This allows for dynamic adjustment of incentives to reflect the DAO's evolving priorities and resources.