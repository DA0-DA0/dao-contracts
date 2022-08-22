# CW721 Controllers: Common cw721 controllers for many contracts

This package provides methods for creating, querying, and completing
claims for cw721s. It is similar to the cw-plus
[cw-controllers](https://crates.io/crates/cw-controllers) package
but it operates on non-fungible tokens instead of fungible ones.x

When you stake a NFT (see the `cw-dao-voting-cw721-stake`) contract
you may later choose to unstake it. If there is a non-zero unstaking
duration, a claim will be created for you. A claim is a piece of state
that entitles you to a particular cw721 NFT at a particular time. In
the unstaking case this is the NFT you are unstaking and
`current_time + unstaking_duration`.
