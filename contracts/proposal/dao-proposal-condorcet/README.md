This is a DAO DAO proposal module which implements The Condorcet
Method

https://www.princeton.edu/~cuff/voting/theory.html

It differs from `dao-proposal-single` and `dao-proposal-multiple` in
that it does not:

1. support revoting,
2. integrate with pre-propose modules, nor
3. support proposal and vote hooks

> But even these three rules  
> needn’t be followed; what works reliably  
> is to know the raw silk,  
> hold the uncut wood.  
> Need little,  
> want less.  
> Forget the rules.  
> Be untroubled.  

- [Tao Te Ching (Ursula Le Guin transaltion)](https://github.com/lovingawareness/tao-te-ching/blob/master/Ursula%20K%20Le%20Guin.md)

The implementation is described below.

# Gas Efficent Ranked Choice Voting

Here we describe a method for selecting the Condorcet winner from a
set of ballots suitable for implementation in a smart contract. To do
so we guarantee that if a proposal can be created it can be voted on
and executed within
[gas](https://ethereum.org/en/developers/docs/gas/#what-is-gas)
limits, by performing tallying in constant time over the number of
votes cast. We provide a complete implementation of this method as a
DAO DAO [proposal
module](https://github.com/DA0-DA0/dao-contracts/wiki/DAO-DAO-Contracts-Design#proposal-modules)
and formally verify the conditions for proposals being passed and
rejected early.

---

## Why Ranked Choice is Hard

The most common form of ranked choice voting, instant run off, [works
like this](https://github.com/DA0-DA0/dao-contracts/discussions/605):

1. Voters submit a list of candidates sorted by their preference.
2. If there is an option with the majority of first-preference votes,
   that option is the winner.
3. Otherwise, remove the option with the fewest first-preference votes
   from all preference lists and repeat.

This algorithm presents a problem: the compute cost for tallying an
election result increases with the number of votes cast (step 3 is at
least $O(voters)$). On a blockchain this presents a problem as voting
power is typically fungible tokens and can be split among many
addresses by a single actor (like a sybil attack). Thus, to stop a
proposal from being executable, an attacker can split their vote among
more and more addresses until the number of votes cast causes tallying
to hit compute limits, making it impossible to pass the proposal.

This is all to say: in order for a ranked choice voting system to be
suitable for use in a smart contract the cost of tallying votes can't
scale with the number of votes cast.

## The Condorcet Method

> Without looking terribly deeply into the history of it, this appears
> to have been first described by Paul Cuff Et al. [1] in 2012. We did
> not come up with this idea.

A [Condorcet
winner](https://en.wikipedia.org/wiki/Condorcet_winner_criterion#:~:text=The%20Condorcet%20winner%20is%20the,candidates%20in%20a%20plurality%20vote.)
is a candidate in an election who would win a 1v1 with every other
candidate. If we assume that voters won't change their relative
preferences when candidates are removed we can find the Condorcet
winner in a set of ranked choice ballots. For example:

```
[a, b, c]
[b, a, c]
[c, a, b]
```

Under this assumption, to see who would win in a 1v1 we can remove all
other candidates from the ballots and compare them using
majority-wins.

```
a vs b  |  a vs c  |  a vs b vs c
        |          |
[a, b]  |  [a, c]  |  [a, b, c]
[b, a]  |  [a, c]  |  [b, a, c]
[a, b]  |  [c, a]  |  [c, a, b]
```

In this example, under the reordering assumption, `a` is a Condorcet
winner.

## Now, In Constant Time

To make this constant time over number of votes ,... mostly just this:
https://gist.github.com/0xekez/e4d3ff76bf76f052af1a8768231831aa then
add info about how we extend this to ensure that if a proposal can be
created it can also be voted on and executed. Also add info about
diagonal and how that gives us $\frac{N(N-1)}{2}$ storage and how we
can map indexes to rows and columns.

## Passing Proposals Early



## The Filibuster

Lead into how this allows for filibustering, but point out that this
can only happen in the direction of no outcome, not towards an
alternative outcome. Describe conditions for filibustering in terms of
passing early.

## Strategic Voting

The Condorcet Method selects a winner from a list of ranked choice
ballots "if there is a candidate who 'wins' EVERY comparison with all
other candidates"[2]. This method is claimed in literature [5] and [6]
online [1] to be quite robust to strategic voting because there are
rare and few circumstances where a voter could "vote insincerely in
order to help elect a preferred candidate"[3].

Briefly summarize [this
paper](https://www.princeton.edu/~cuff/publications/wang_strategic_voting.pdf)
which shows that strategic voting will only happen along boundaries of
no winner. Tie this in with filibuster which is an example of this
happening. Admit ignorance and possibility of unknown unknowns. Also
see the "Robust to Voters (Strategic voting)" section
[here](https://www.princeton.edu/~cuff/voting/theory.html).

## Rejecting Proposals Early

https://github.com/DA0-DA0/dao-contracts/wiki/Proofs-of-early-rejection-cases-for-Condorcet-proposals

## Conclusion

Repeat of the abstract but in different words and using more detail
and terminology.

[1]: https://www.princeton.edu/~cuff/voting/theory.html
[2]: https://web.math.princeton.edu/math_alive/Voting/Lab1/Condorcet.html
[3]: https://www.princeton.edu/~cuff/publications/wang_strategic_voting.pdf
[5]: http://www.princeton.edu/~cuff/publications/wang_allerton_2012.pdf
[6]: https://www.princeton.edu/~cuff/publications/cuff_nips_2012.pptx
