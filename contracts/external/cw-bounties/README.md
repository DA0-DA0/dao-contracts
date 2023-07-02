# cw-bounties

A simple bounties smart contract. The contract is instantiated with an owner who controls when bounties are payed out (usually a DAO).

On bounty creation the funds are taken, on update funds are added or removed and bounty details can be updated, on removal funds are returned to the bounties contract owner.

Typical usage would involve a SubDAO with open proposal submission. Bounty hunters would be able to see a list of bounties, work on one and make a proposal to claim it.
