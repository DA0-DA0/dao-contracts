# CW4 Group Voting

A simple voting power module which determines voting power based on
the weight of a user in a cw4-group contract. This allocates voting
power in the same way that one would expect a multisig to.

For more information about how these modules fit together see
[this](https://github.com/DA0-DA0/dao-contracts/wiki/DAO-DAO-v1-Contracts-Design)
wiki page.

## Receiving updates

This contract does not make subqueries to the cw4-group contract to
get an addresses voting power. Instead, it listens for
`MemberChangedHook` messages from said contract and caches voting
power locally.

As the DAO is the admin of the underlying cw4-group contract it is
important that the DAO does not remove this contract from that
contract's list of hook receivers. Doing so will cause this contract
to stop receiving voting power updates.
