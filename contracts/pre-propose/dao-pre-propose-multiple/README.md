# Multiple choice proposal deposit contract

[![dao-pre-propose-multiple on crates.io](https://img.shields.io/crates/v/dao-pre-propose-multiple.svg?logo=rust)](https://crates.io/crates/dao-pre-propose-multiple)
[![docs.rs](https://img.shields.io/docsrs/dao-pre-propose-multiple?logo=docsdotrs)](https://docs.rs/dao-pre-propose-multiple/latest/dao_pre_propose_multiple/)

This is a pre-propose module that manages proposal deposits for the
`dao-proposal-multiple` proposal module.

It may accept either native ([bank
module](https://docs.cosmos.network/main/modules/bank/)),
[cw20](https://github.com/CosmWasm/cw-plus/tree/bc339368b1ee33c97c55a19d4cff983c7708ce36/packages/cw20)
tokens, or no tokens as a deposit. If a proposal deposit is enabled
the following refund strategies are avaliable:

1. Never refund deposits. All deposits are sent to the DAO on proposal
   completion.
2. Always refund deposits. Deposits are returned to the proposer on
   proposal completion.
3. Only refund passed proposals. Deposits are only returned to the
   proposer if the proposal passes. Otherwise, they are sent to the
   DAO.

This module may also be configured to only accept proposals from
members (addresses with voting power) of the DAO.

Here is a flowchart showing the proposal creation process using this
module:

![](https://bafkreibymt3n6avrpdeukwqplw366yyk5cgrrjtwszib2hk2updmyy7apa.ipfs.nftstorage.link/)


### Resources

More about the [pre-propose design](https://github.com/DA0-DA0/dao-contracts/wiki/Pre-propose-module-design).

More about [pre-propose modules](https://github.com/DA0-DA0/dao-contracts/wiki/DAO-DAO-Contracts-Design#pre-propose-modules).
