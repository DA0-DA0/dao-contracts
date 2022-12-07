# Private Voting

This module is work in progress.
It aims to be able to wrap a module that can return the voting power per address (and voting power updates).
Then it can take user messages as both shielded sender and public-sender votes.
It streams vote updates to a "tallyer" module.

Theres two high level approaches to private voting:

1) Obfuscator approach: Anonymize all voters, but have the individual votes fully public.
2) Homomorphically encrypt all votes, publicly sum them all, and have a trusted party decrypt the sum.
    * Can combine with (1) to publicly verify that no double voting occured
    * Trusted party could be a threshold set

This module implements (1), and leaves it to the future to progress on (2), as they generically compose.
Ferveo ([paper](https://eprint.iacr.org/2022/898), [github](https://github.com/anoma/ferveo)) could likely be adapted to work to make (2) work in the threshold setting. (Would need to mildly adapt the encryption scheme to be homomorphic).

## High level overview

The contract supports both public and private voting. The flow if you want private voting is as follows:

* Suppose you have 10 units of stake.
* You do a tx to the private voting contract, that gives you 10 ballots, each with an ID publicly attached to you. You are no longer able to do public voting, until you invalidate these ballots.
* To vote on proposal `P`, you submit one tx (from any address) per ballot, which contains a ZKP attesting:
  * "This ballot is used to vote `V` on proposal `P`" (public)
  * "This message was created by the owner of ballot ID `B`" (in zero knowledge)
  * "`B` has not voted on proposal `P`" (in zero knowledge)
  * "`B` has been added to the set of ballots who have voted on proposal `P`" (in zero knowledge)s

To change your vote from `V -> V'`, you do a pretty similar proof. Instead you prove that ballot `B` has already voted `V` on proposal `P`, and you want to delete that record, and instead put `V'`.

## Underlying voting module expectations

We expect there to be an underlying voting contract, that has the following method:

```rust
pub trait VoteModule {
  fn voting_power_by_addr(&self, addr: Addr) -> Option<Uint128>
}
```

And it must have a "hook" where after any voting power update, it can call a method on the private voting contract.
I don't know how you typically handle circular dependencies like this in Rust, presumably we extract the interfaces for wiring into some common crate both depend on?

So this may look like:

```rust
pub trait VoteModule {
  fn add_voting_power_update_subscriber(&mut self, sub: VotingPowerUpdateSubscriber)
}

pub trait VotingPowerUpdateSubscriber {
  fn on_voting_power_update(&self, deps: DepsMut, addr: Addr, vp_update: Int128) -> Result<(), ContractError>
}
```

## Vote tallying module expectations

For MVP'ing, we start by assuming votes have a binary predicate. (Yes/No)

But we want to generalize, to allow proposals to have:

1) Allow delegation of votes
2) Have more complex decision rules (Approval voting, Cosmos-gov style voting, Instant run-off, etc)

To enable (1), we introduce a concept of "voter_sets". Some examples of voting sets: (Perhaps consider the name voter anonymity set?)

* If I vote directly, count my vote. Otherwise don't delegate my vote.
* If I vote directly, count my vote directly. Otherwise delegate my vote to FOOBAR.

We consider every distinct address you delegte to as a new voter set. Your vote can only gain anonymity within the voter set they are in.

To enable (2), we add a template parameter describing the vote type. Each vote type must have an ID. (E.g. Yes = 1, No = 2, etc.). TODO: Explain this in more detail.

The private voting module provides the following subscriptions for the vote tallying module to listen onto:

```rust
// TODO
```

The vote tallying module here is restricted relative to public voting. In the current privacy model, where we hide the voter from everyone, we actually can't fairly handle voting power changes during the voting period. So the voting power a shielded voter has, must either:

* Be frozen at proposal start time
* Allow for people who shielded vote before their voting power decreases, to unfairly 'get away' with their already cast votes.

For simplicity in MVP, we assume the latter, but this should be simple to change to *frozen at proposal start time*.

### Metadata leakage

So now we have voting that is anonymized within a voter set! But there is still some significant metadata leakage that must be thought about.

The units of information that can be used to try to infer who the voter is:

* How far apart were a user's votes in time?
* How many ballots does a user have & how many votes in agreement were submitted?
* Who was the transaction sender?
* Did the ballots vote on multiple proposals around the same time

We discuss some ideas for hiding these below. (TODO)

#### Hiding the Tx sender

One key point is "who submits the vote transaction".

As an MVP, we can have the sender of the tx executing the vote be a centralized server you communicate with.
But once Namada launches, we can bootstrap this off of their anonymity set. What we'd do is have the user have some tokens on Namada for paying fees.
Then they create a tx on Namada from an anonymized sender, that IBC's to the chain with the private-voting contract, and uses [`ibc-wasm-hooks`](https://github.com/osmosis-labs/osmosis/blob/v13.x/x/ibc-hooks/README.md) to execute the private vote on this chain.
