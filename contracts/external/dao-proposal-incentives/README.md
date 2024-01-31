# dao-proposal-incentives

[![dao-proposal-incentives on crates.io](https://img.shields.io/crates/v/dao-proposal-incentives.svg?logo=rust)](https://crates.io/crates/dao-proposal-incentives)
[![docs.rs](https://img.shields.io/docsrs/dao-proposal-incentives?logo=docsdotrs)](https://docs.rs/dao-proposal-incentives/latest/cw_admin_factory/)

Allows for DAOs to offer incentives for making successful proposals.

To setup this contract, the DAO needs to add this contract as a `ProposalHook` to the `dao-voting-single` or `dao-voting-multiple` proposal module, and the DAO must be the `owner` of this contract. When someone successfully passes a proposal the specified rewards are automatically paid out.

The incentives can be configured as native or cw20 tokens, and the award is determined by the configuration at the passed proposal's `start_time`.