# cw-fund-distributor

This contract is meant to facilitate fund distribution 
proportional to the amount of voting power members have
at a given block height.

Possible use cases may involve:
- Dissolving a DAO and distributing its treasury to members prior to shutting down
- Distributing funds among DAO members
- Funding subDAOs

## Funding Period

Contract is instantiated with a `funding_period` - a time duration that should suffice 
to move the funds to be distributed into the distributor contract.

Funding the contract can only happen during this period.
No claims can happen during this period.

## Claiming/Distribution Period

After the `funding_period` expires, the funds held by distributor contract become
available for claims.

Funding the contract is no longer possible at this point.

## Fund redistribution

Considering it is more than likely that not every user would claim its allocation,
it is possible to redistribute the unclaimed funds.

Only the `cw_admin` can call the method.

The redistribution method finds all the claims that have been performed
and subtracts the amounts from the initially funded balance. The respective 
allocation ratios for each DAO member remain the same; any previous claims
are cleared.
