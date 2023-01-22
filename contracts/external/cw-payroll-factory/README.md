# cw-payroll-factory

Serves as a factory that instantiates [cw-vesting](../cw-vesting) contracts and stores them in an indexed maps for easy querying by recipient or the instantiator (i.e. give me all of my vesting payment contracts or give me all of a DAO's vesting payment contracts).

An optional `owner` can be specified when instantiating `cw-payroll-factory` that limits contract instantiation to a single account.
