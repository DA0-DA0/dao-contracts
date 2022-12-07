# cwd-proposal-single

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

## Undesired behavior

The undesired behavior of this contract is tested under `testing/adversarial_tests.rs`.

In general, it should cover:
- Executing unpassed proposals
- Executing proposals more than once
- Social engineering proposals for financial benefit
- Convincing proposal modules to spend someone else's allowance

## Proposal deposits

Proposal deposits for this module are handled by the
[`cwd-pre-propose-single`](../../pre-propose/cwd-pre-propose-single)
contract.

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

## Revoting

The proposals may be configured to allow revoting.
In such cases, users are able to change their vote as long as the proposal is still open.
Revoting for the currently cast option will return an error.
