# `dao_voting_cosmos_staking`

A DAO DAO voting contract that uses Cosmos SDK staking for calculating voting power.

For example, if I stake 100 Juno, I have 100 Juno worth of voting power in the DAO.

## Naive Approach: Mirror with historical data

Just query for info...

Problem: no historical info.

```ignore
        QueryMsg::VotingPowerAtHeight { address, height } => {
            // Check if is historical data
            let staking_history = HISTORICAL_STAKING_SNAPSHOT.may_load(address, height)
            // If no historical data, query stargate delegation info (just ignore height as long as it's less than unbonding period)
            match staking_history {
                // IF historical data, we use that 
                Some(hist) => Ok(hist),
                // else do a stargate query, user's staking balance hasn't changed since snapshotting began
                None => {
                    // Stargate Query!
                }
            }
        }
        QueryMsg::TotalPowerAtHeight { height } => {
            // TODO query total power (store as snapshot map)
        }



        // cw-hooks
        SudoMsg::AfterDelegationModified {validator: String, delegator: String, shares: String} => {
            // if delegator is in a pending proposals vote list, update theri vote
            HISTORICAL_STAKING_SNAPSHOT.save(info)
        }
```


# on change
- user votes, query current VP delegated
- cw-hooks update the previous VP change amount


When a prop is created, we could have a hook that stores current total voting power at proposal start time.
Use clock to fire membership changed events.

## Solution with Clock

- Every end of block, the `SudoMsg::ClockEndBlock` will be called by the chain (when registered).
- execute the proposal then
- only direct votes (no validator overrides)
- always have quaroum? (DAO config to allow for % of total, OR instant quaroum based off of the voted accounts)
- execute it if end


- 2bn gas limit, assume this for the contract.
- SudoMsg::ClockEndBlock
    - iter all proposals currently in voting period
    - if proposal is expired, execute it
    - on exec, query stake from stargate for everyone who voted
    - sum this up, then perform based off the config (quaroum, % of total who did vote, etc)



# flow
- put up a proposal, text
- people vote on it, we ONLY save their juno address to a list / map (we do not care about VP)
- execute closes voting period, expire (pre tally)
- ClockEndBlock sees this, then queries ALL balances from the list, tallies it up, and executes if




