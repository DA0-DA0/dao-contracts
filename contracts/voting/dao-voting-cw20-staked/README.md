# CW20 Staked Balance Voting

A voting power module which determines voting power based on the
staked token balance of specific addresses at given heights.

Also features the functionality to set an active threshold, this
threshold allows DAOs to be marked as inactive if it is not met.  This
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
