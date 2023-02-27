
We want the following to be true:

1. I can create a vesting agreement between two parties.
2. The owner of the agreement may cancel it at any time and reclaim
   unvested funds.
3. The receiver of the tokens may stake those tokens on the underlying
   cosmos-SDK blockchain and receive rewards.

Requirement two means that:

1. Funds should never be paid out faster than scheduled.
2. Already vested funds should never be returned to the owner.

Thrown into the mix by requirement three, is a major complication: the
SDK does not provide hooks when slashing happens. Later I will show
that this means that there is a situation where the contract does not
have enough information to enforce that on cancelation vested funds
are always returned to the receiver. See
`test_owner_registers_slash_after_withdrawal` in
`src/suite_tests/tests.rs` for a test which demonstrates this.

How do we know our requirements have been met?

- `src/vesting_tests.rs` tests that the rules of vesting are followed
  (1 and 2).
- `src/stake_tracker_tests.rs` tests that staked balances are tracked
  properly (3).
- `src/suite_tests/tests.rs` tests that the whole system works well
  together in some complex scenerios.
- `src/tests.rs` has some additional integration tests from an earlier
  iteration of this contract.
- `ci/integration-tests/src/tests/cw_vesting_test.rs` tests a bond,
  withdraw rewards, unbond flow with this contract to ensure that it
  behaves correctly against a real cosmos-SDK blockchain. This test is
  important because cw-multi-test has some bugs in its x/staking
  implementaiton. Tests demonstrating these can be found in
  `test_slash_during_unbonding` and `test_redelegation` in the suite
  tests.

## Slashing

Slashing can happen while tokens are staked to a validator, or while
tokens are unbonding from a validator (ref: unbonding durations
protect against long range attacks). Let's investigate how slashing
impacts this contract.

In this contract we use two math formulas, $liquid(t)$ tells us the
contract's current liquid token balance, and $claimable(t)$ tells us
how many tokens may be claimed by the vest receiver. As the vest
receiver selects which validators to delegate to, we expect that they
will be penalized for slashing, and not the vest owner. This makes our
formulas:

$$ liquid(t) := total - claimed(t) - staked(t) - slashed(t) $$

$$ claimable(t) := vested(t) - claimed(t) - slashed(t) $$

The Cosmos SDK does not provide a way for contracts to be notified
when a slash happens, so let's consider what happens if a slash occurs
and the contract does not know about it.

### Slashed while staked

If tokens are slashed while they are staked and the contract does not
know of it, it will make $staked(t) = staked(t) + slashed(t)$, as the
contract will not know of the slash and thus will not deduct it from
the staked balance. This means that $liquid$ will continue to return
correct values.

$claimable$ on the other hand will report values $slashed(t)$ too
large. this has different impacts depending on if the contract is
canceled or not.

#### Slash while staked, contract open

The amount of funds to distribute is $min(liquid(t),
claimable(t))$. This means that in the time following the slash
claimable will be $slashed(t)$ too large and the receiver will be able
to withdraw "too much"; however, once the vest completes $liquid(t)$
being correct will stop any claiming of funds $\gt total$.

#### Slashed while staked, contract cancelled

When a contract is canceled currently liquid funds are sent to the
vest receiver up to the amount that they may claim and, $total$ is set
to $vested(t)$.

$$ settle = min(claimable(t), liquid(t)) $$

Because $claimable(t)$ is $slashed(t)$ too large, **a closed contract
with a slash may distribute too many tokens to the vest
receiver**. This makes the owner bear the cost of shashing.

### Slashed while unbonding

If tokens are slashed while they are unbonding this will make
$liquid(t)$ $slashed(t)$ too large, and $claimable(t)$ $slashed(t)$
too large.

#### Slashed while unbonding, contract open

This has the same outcome as being slashed while bonded and open as
the factory pattern will prevent a vesting contract from being able to
distribute more tokens than it has (x/bank to the rescue).

#### Slashed while unbonding, contract closed

The contract will not be closable as the overestimate of $liquid(t)$
will cause the contract to attempt to settle more funds than it has
(x/bank will error).

### Slashing conclusion

If slashes are not known about, it can cause bad-ish outcomes. After
much discussion, we decided that these are acceptable. This contract
also provides a message type `RegisterSlash` which allows the owner to
register a slash that has occured and in doing so rebalance the
contract to undo the issues discussed above.
