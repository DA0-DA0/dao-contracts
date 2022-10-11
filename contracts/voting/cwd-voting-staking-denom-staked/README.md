# CW Native Staked Balance Voting

This voting module determines voting power based on staked native
tokens in the bank module. You could, for example, use this to allow
Juno stakers to vote on proposals using the same voting power they use
when doing native SDK governance votes.

~WARNING~ This module behaves differently than other DAO DAO voting
modules. Specifically:

1. Staking tokens after a proposal has been created will allow you to
   vote on that proposal with your new staked balance.
2. Voting before staking tokens and then staking tokens will cause
   your vote to be registered with a lower voting power. DAOs with
   revoting will allow casting a vote with the new voting power.
3. Proposal modules MUST never have a voting duration longer than the
   chains unbonding time, otherwise double voting will be possible.

When instantiating this contract, you must use the module address of
the staking module as the `staking_module_address` field. Smart
contracts can not validate that the address used is actually the
staking module address, so the burden of correctness is entirely on
the module's creator.
