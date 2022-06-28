# cw-proposal-single

A proposal module for a DAO DAO DAO which supports simple "yes", "no",
"abstain" voting. Proposals may have associated messages which will be
executed by the core module upon the proposal being passed and
executed.

For more information about how these modules fit together see
[this](https://github.com/DA0-DA0/dao-contracts/wiki/DAO-DAO-v1-Contracts-Design)
wiki page.

For information about how this module counts votes and handles passing
thresholds see
[this](https://github.com/DA0-DA0/dao-contracts/wiki/A-brief-overview-of-DAO-DAO-voting#proposal-status)
wiki page.

## Proposal deposits

This contract may optionally be configured to require a deposit for
proposal creation. Currently, any cw20 token may be used.

As a convienence one may specify that the module should use the same
token as the DAO's voting module using the `VotingModuleToken` variant
when specifying information about the deposit. For this to work the
voting module associated with the DAO must support the `TokenContract`
query. This query may be derived via the `#[token_query]`
[macro](../../packages/cw-core-macros/src/lib.rs).

## Hooks

This module supports hooks for voting and proposal status changes. One
may register a contract to receive these hooks with the `AddVoteHook`
and `AddProposalHook` methods. Upon registration the contract will
receive messages whenever a vote is cast and a proposal's status
changes (for example, when the proposal passes).

The format for these hook messages can be located in the
`proposal-hooks` and `vote-hooks` packages located in
`packages/proposal-hooks` and `packages/vote-hooks` respectively.

To stop an invalid hook receiver from locking the proposal module
receivers will be removed from the hook list if they error when
handling a hook.
