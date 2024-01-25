# dao-voting-incentives

[![dao-voting-incentives on crates.io](https://img.shields.io/crates/v/dao-voting-incentives.svg?logo=rust)](https://crates.io/crates/dao-voting-incentives)
[![docs.rs](https://img.shields.io/docsrs/dao-voting-incentives?logo=docsdotrs)](https://docs.rs/dao-voting-incentives/latest/cw_admin_factory/)

Allows for DAOs to offer incentives for voting on DAO proposals.

When creating this contract, the DAO specifies an `epoch_duration` and an amount to pay out per epoch. Then, the DAO needs to add this contract as a `VoteHook` to the `dao-voting-single` or `dao-voting-multiple` proposal module. When DAO members vote, this contract keeps track of the proposals and who voted.

At the end of the epoch, rewards are payable as follows:

```
rewards = (user vote count / prop count) / total_vote_count * voting incentives
```

If no proposals happen during an epoch, no rewards are paid out.

## TODO
- [ ] Unit and Integration tests with a full DAO
- [ ] Make sure it works with multiple proposal modules (i.e. multiple choice and single choice)
- [ ] Make sure claiming rewards is gas effecient even if many epochs have passed.
- [ ] Support Cw20.
- [ ] Use `cw-ownable` to configure a contract owner who can update the voting incentives config.
- [ ] Add more info to the readme and delete this TODO section.
