# Private Voting

This module is work in progress.
It aims to be able to wrap a module that can return the voting power per address (and voting power updates).
Then it can take user messages as both shielded sender and public-sender votes.
It streams vote updates to a "tallyer" module.

Theres two high level approaches to private voting:

1) Obfuscator approach: Anonymize all voters, but have the individual votes fully public.
2) Homomorphically encrypt all votes, publicly sum them all, and have a trusted party decrypt the sum.
    * Use (1) to publicly verify that no double voting occured
    * Trusted third party could be a threshold set

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

### Making delegation by default work

If you want delegation to work, your vote can only gain anonymity within the set of people who are delegated to the same person.

### Metadata leakage

So now we have voting that is anonymized! But there is still some significant metadata leakage that must be thought about.

The units of information that can be used to try to infer who the voter is:

* How far apart were a user's votes in time?
* How many ballots does a user have & how many votes in agreement were submitted?
* Who was the transaction sender?

We discuss some ideas for hiding these below. (TODO)

#### Hiding the sender

One key point is "who submits the vote proposal".

As an MVP, we can have the sender of the tx executing the vote be a centralized server you communicate with.
But once Namada launches, we can bootstrap this off of their anonymity set. What we'd do is have the user have some tokens on Namada for paying fees.
Then they create a tx on Namada from an anonymized sender, that IBC's to the chain with the private-voting contract, and uses [`ibc-wasm-hooks`](https://github.com/osmosis-labs/osmosis/blob/v13.x/x/ibc-hooks/README.md) to execute the private vote on this chain.
