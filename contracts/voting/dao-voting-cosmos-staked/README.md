# Cosmos SDK Staked Balance Voting

> **WARNING**: This contract is experimental and should not be used to govern
> significant assets nor security-critical procedures. Due to the limitations of
> the Cosmos SDK and CosmWasm, this voting module cannot provide the guarantees
> needed by a fully secure DAO. More on this below.

A DAO DAO voting contract that uses Cosmos SDK staking for calculating voting
power. This allows a DAO to mimic members' stake in the chain (and thus voting
power in chain governance props).

## Limitations

Unfortunately, the Cosmos SDK does not currently store historical staked
amounts, so this module suffers from some limitations.

### Voter's staked amount

Voting power for a voter is always calculated based on the current amount staked
(regardless of which block is requested in the query) since there is no
historical info. Since proposal modules query and save this value when a voter
casts a vote, the voting power used is frozen at the time of voting.

If revoting is not allowed, a voter may be incentivized to wait for others to
vote, acquire more voting power, and vote once the others cannot change their
voting power.

If revoting is allowed, voting power is updated when a voter changes their vote.
This opens up the possibility for a voter to manipulate the perceived outcome
while a proposal is still open, changing their voting power and revoting at the
last minute to change the outcome.

Cosmos SDK governance operates the same way—allowing for voting power to change
throughout a proposal's voting duration—though it at least re-tallies votes when
the proposal closes so that all voters have equal opportunity to acquire more
voting power.
