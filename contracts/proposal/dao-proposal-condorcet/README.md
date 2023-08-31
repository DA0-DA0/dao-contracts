# dao-proposal-condorcet

[![dao-proposal-condorcet on crates.io](https://img.shields.io/crates/v/dao-proposal-condorcet.svg?logo=rust)](https://crates.io/crates/dao-proposal-condorcet)
[![docs.rs](https://img.shields.io/docsrs/dao-proposal-condorcet?logo=docsdotrs)](https://docs.rs/dao-proposal-condorcet/latest/dao_proposal_condorcet/)

This is a DAO DAO proposal module which implements The Condorcet
Method.

https://www.princeton.edu/~cuff/voting/theory.html

This module lacks many basic features. For example, proposals and
choices do not have human readable names and descriptions. For this
first version, the goal is to build a correct, secure, and gas
efficent voting system that may be audited, not to build a proposal
module that is ready for use with humans and a frontend.

To this end, this module differs from `dao-proposal-single` and
`dao-proposal-multiple` in that it does not:

1. support revoting,
2. integrate with pre-propose modules, nor
3. support proposal and vote hooks

The ranked choice voting system used is described in detail
[here](./gercv.pdf). This contract will make no sense unless you read
that PDF first as there is a fair bit of math.

> what works reliably  
> is to know the raw silk,  
> hold the uncut wood.  
> Need little,  
> want less.  
> Forget the rules.  
> Be untroubled.  

- [Tao Te Ching (Ursula Le Guin transaltion)](https://github.com/lovingawareness/tao-te-ching/blob/master/Ursula%20K%20Le%20Guin.md)

