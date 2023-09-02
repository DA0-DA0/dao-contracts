# CW20 Staked Balance Voting

[![dao-voting-cw20-staked on crates.io](https://img.shields.io/crates/v/dao-voting-cw20-staked.svg?logo=rust)](https://crates.io/crates/dao-voting-cw20-staked)
[![docs.rs](https://img.shields.io/docsrs/dao-voting-cw20-staked?logo=docsdotrs)](https://docs.rs/dao-voting-cw20-staked/latest/dao_voting_cw20_staked/)

A voting power module which determines voting power based on the
staked token balance of specific addresses at given heights.

This contract implements the interface needed to be a DAO
DAO [voting
module](https://github.com/DA0-DA0/dao-contracts/wiki/DAO-DAO-Contracts-Design#the-voting-module).
It also features the functionality to set an active threshold, this
threshold allows DAOs to be marked as inactive if it is not met. This
threshold can either be an absolute count of tokens staked or a
percentage of the token's total supply.

## Endpoints

### Execute

`UpdateActiveThreshold` - Allows the user to update the active
threshold.

### Query

`TokenContract` - Provided via the `token_query` macro, simply returns
the underlying CW20 token's address.

`StakingContract` - Returns the underlying staking contract used to
derive voting power at a given height. Should point to an instance of
`cw20-stake`.

`VotingPowerAtHeight` - Given an address and an optional height,
return the voting power that address has at that height. If no height
is given it defaults to the current block height. In this case it is
the address' staked balance at that height.

`TotalPowerAtHeight` - Given an optional height, determine the total
voting power available. If no height is given it defaults to the
current block height.  In this case it is the total staked balance at
that height.

`Info` - Uses the CW2 spec to return the contracts info.

`Dao` - Returns the DAO that this voting module belongs to.

`IsActive` - Returns true or false depending on if this DAO is active
and can make proposals. Uses the active threshold described above to
determine this.

`ActiveThreshold` - Returns the details for the current active
threshold in place, if any.
