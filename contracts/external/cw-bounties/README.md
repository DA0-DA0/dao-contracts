# cw-bounties

A simple bounties smart contract. The contract is instantiated with an owner who controls when bounties are payed out (usually a DAO).

NOTE: this contract has NOT BEEN AUDITED and is not recommended for production use. Use at your own risk.

## Overview

On `create` the bounty funds sent along with the transaction are taken and held in escrow.

On `update` funds are added or removed and bounty details can be updated. If the updated amount is less than the original amount, or if the `denom` for the payout has changed (for example, switching from $USDC to $JUNO), funds will be returned to the contract owner (again, usually a DAO).

On `close` funds are returned to the bounties contract owner.

Typical usage would involve a DAO DAO SubDAO with open proposal submission. Bounty hunters would be able to see a list of bounties, work on one and make a proposal to claim it.

## Future work
- [ ] Support partial claims (i.e. I did some meaninful work but didn't finish the bounty, so claiming only part of it).
- [ ] Support bounties with multiple claims (i.e. a task with the first three people to complete it pays out an equal reward to all).
