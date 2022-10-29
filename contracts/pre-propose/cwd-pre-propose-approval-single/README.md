# Single choice proposal approval contract

This contract implements an approval flow for proposals, it also handles deposit logic. It works with the `cwd-proposal-single` proposal module.

## Approval Logic

This contract is instantatied with an `approver` address. This address is allowed to call methods on this contract (approve / reject / add hook / remove hook).

### Queries

TODO what queries should this support?

## Approver logic

The defined approver can call approve or reject on the `pre-propose-approval-flow module`.

To improve UX, an approver contract can be instantiated. This registers a hook with the approval contract to automatically create proposals in the approver DAO.

When SubDAO recieves prop as hook, it makes an approval prop. If approved, the approval prop calls the approve message on this contract. When prop fails it fires off reject call.

### Open Questions
QUESTION: is this contract the same for the approver? Probably cleaner to have it separate... Some methods aren't needed like approve and reject. Maybe just don't allow extension when admin?
What happens with deposit? Goes to DAO? Pre-propose approver maybe should not allow deposits?

## Deposit Logic

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

![](https://bafkreig42cxswefi2ks7vhrwyvkcnumbnwdk7ov643yaafm7loi6vh2gja.ipfs.nftstorage.link)

