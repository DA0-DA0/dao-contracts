## cwd-proposal-multiple

A proposal module for a DAO DAO DAO which allows the users to select
their voting choice(s) from an array of `MultipleChoiceOption`.
Each of the options may have associated messages which are to be
executed by the core module upon the proposal being passed and executed.

## Undesired behavior

The undesired behavior of this contract is tested under `testing/adversarial_tests.rs`.

In general, it should cover:
- Executing unpassed proposals
- Executing proposals more than once
- Social engineering proposals for financial benefit
- Convincing proposal modules to spend someone else's allowance

## Proposal deposits

Proposal deposits for this module are handled by the
[`cwd-pre-propose-multiple`](../../pre-propose/cwd-pre-propose-multiple)
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
