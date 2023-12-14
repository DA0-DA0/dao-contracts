# dao-proposal-single

[![dao-proposal-single on crates.io](https://img.shields.io/crates/v/dao-proposal-single.svg?logo=rust)](https://crates.io/crates/dao-proposal-single)
[![docs.rs](https://img.shields.io/docsrs/dao-proposal-single?logo=docsdotrs)](https://docs.rs/dao-proposal-single/latest/dao_proposal_single/)

A proposal module for a DAO DAO DAO which supports simple "yes", "no",
"abstain" voting. Proposals may have associated messages which will be
executed by the core module upon the proposal being passed and
executed.

Votes can be cast for as long as the proposal is not expired. In cases
where the proposal is no longer being evaluated (e.g. met the quorum and
been rejected), this allows voters to reflect their opinion even though 
it has no effect on the final proposal's status.

For more information about how these modules fit together see
[this](https://github.com/DA0-DA0/dao-contracts/wiki/DAO-DAO-Contracts-Design)
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
[`dao-pre-propose-single`](../../pre-propose/dao-pre-propose-single)
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

## Veto

Proposals may be configured with an optional `VetoConfig` - a configuration describing
the veto flow.

VetoConfig timelock period enables a party (such as an oversight committee DAO)
to hold the main DAO accountable by vetoing proposals once (and potentially
before) they are passed for a given timelock period.

No actions from DAO members are allowed during the timelock period.

After the timelock expires, the proposal can be executed normally.

`VetoConfig` contains the following fields:

### `timelock_duration`

Timelock duration (`cw_utils::Duration`) describes the duration of timelock
in blocks or seconds.

The delay duration is added to the proposal's expiration to get the timelock
expiration (`Expiration`) used for the new proposal state of `VetoTimelock {
expiration: Expiration }`.

If the vetoer address is another DAO, this duration should be carefully
considered based on of the vetoer DAO's voting period.

### `vetoer`

Vetoer (`String`) is the address of the account allowed to veto the proposals
that are in `VetoTimelock` state.

Vetoer address can be updated via a regular proposal config update.

If you want the `vetoer` role to be shared between multiple organizations or
individuals, a
[cw1-whitelist](https://github.com/CosmWasm/cw-plus/tree/main/contracts/cw1-whitelist)
contract address can be used to allow multiple accounts to veto the prop.

### `early_execute`

Early execute (`bool`) is a flag used to indicate whether the vetoer can execute
the proposals before the timelock period is expired. The proposals still need to
be passed and in the `VetoTimelock` state in order for this to be possible. This
may prevent the veto flow from consistently lengthening the governance process.

### `veto_before_passed`

Veto before passed (`bool`) is a flag used to indicate whether the vetoer
can veto a proposal before it passes. Votes may still be cast until the
specified proposal expiration, even once vetoed.
