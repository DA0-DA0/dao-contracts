# Gauge Orchestrator Contract

There are many places where we want something like a [gauge contract](https://resources.curve.fi/reward-gauges/gauge-weights),
when we need to select a weighted group out of a larger group of options. For example, we will need one to
select the validators that the staking derivatives delegates to, and relative percentages to which one.
We will need one to select which AMM Pools should receive how many incentives (both JUNO and possibly WYND).
We may need one to assign incentives on our lending protocol.

## Orchestrator

To work properly, the gauge must be informed every time that the voting power of a member changes.
It does so by listening to "update hooks" on the underlying staking contract and if an address's
voting power changes, updating their vote weight in the gauge, and the tally for the option they
had voted for (if any).

Every contract call has some overhead, which is silently added to the basic staking action.
If we have 5 gauges in WYND DAO, we would likely have a minimum of 5 x 65k or 325k gas per staking action,
just to update gauges. This is a lot of overhead, and we want to avoid it.

To do so, we make one "Gauge Orchestrator", which can manage many different gauges. They all have the
same voting logic and rules to update when the voting power changes. The Orchestrator is the only
contract that must be called by the staking contract, and doing a few writes for each gauge is a
lot cheaper gas-wise than calling a separate contract.

The Orchestrator has an "owner" (the WYND DAO) which is responsible for adding new gauges here,
and eventually stopping them if we don't need them anymore (to avoid extra writes).

## Gauge Functionality

A gauge is initialised with a set of options. Anyone with voting power may vote for any option at any time,
which is recorded, and also updates the tally. If they revote, it checks their last vote to reduce power on
that before adding to the new one. When an "update hook" is triggered, it updates the voting power
of that user's vote,  while maintaining the same option. Either increasing or decreasing the tally
for the given option as  appropriate.

Every epoch (eg 1/week), the current tally of the gauge is sampled, and some cut-off applies
(top 20, min 0.5% of votes, etc). The resulting set is the "selected set" and the options along with
their relative vote counts (normalised to 1.0 = total votes within this set) is used to initiate some
action (eg. distribute reward tokens).

## Extensibility

We will be using one Orchestrator for many different gauges that update many different contracts.
To make it more extensible, we define option as an arbitrary string that makes sense to that contract.
We also store the integration logic in an external contract, called a `GaugeAdapter` that must provide
3 queries to the Orchestrator:

* Provide set of all options: maybe expensive, iterate over all and return them. This is used for initialization.
* Check an option: Allow anyone to propose one, and this confirms if it is valid (eg is this a valid address
  of a registered AMM pool?)
* Create update messages: Accepts "selected set" as argument, returns `Vec<CosmosMsg>` to be executed by the
  gauge contract / DAO.

### Adapters

We will create a mock implementation of an Adapter for testing.

In production, we will need one for that queries an AMM Factory for open pools,
and knows how to send the rewards to the appropriate pools.

We will need another for a JUNO staking derivative, to select which validators should
be in the set, and then upon execute, inform the contract to delegate to those validators.

We would need a modified one, that uses IBC packets, for a remote staking derivative.
It would need to query the remote chain for the set of validators. And the messages
would be ICA Packets to transmit the new set to the remote chain.

As you can see, it should be a quite flexible design, while keeping the tallying logic
centralized here and minimal gas impact on the staking contract to track the multiple gauges.s

## Example Use

When the DAO wants to add another gauge, it first uploads the code for generating eg. AMM reward messages,
and instantiates a properly configured Adapter. Then, it votes to create a new Gauge that uses this adapter.
Upon creating the gauge, it will query the adapter for the current set of options to initialize state.

After one epoch has passed, anyone can trigger `Execute` on this gauge ID, and the Orchestrator will
apply the logic to determine the "selected set". It will then query the adapter for the messages
needed to convert that selection into the appropriate action, and it will send those to the
[WYND DAO core module](https://github.com/DA0-DA0/dao-contracts/wiki/DAO-DAO-Contracts-Design#the-core-module)
to be executed.

## Storage

Every gauge that is created is given a new auto-incrementing ID.

All non-global state in the contract (only owner and voting power contract) is indexed
first by the gauge and then by the other key (eg. voter address for Votes, option for tallied power, etc)

We do not know how many gauges will be there a priori and this composite index allows us to
be flexible. Not the use of `.prefix()` and `.sub_prefix()` in `state.rs` tests to efficiently
focus on the relevant data for one gauge.
