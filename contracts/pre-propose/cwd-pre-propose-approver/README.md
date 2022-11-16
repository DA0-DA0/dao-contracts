# Proposal approver contract

This contract works in conjuction with `cwd-pre-propose-approval-flow` and allows for automatically creating approval proposals when a proposal is submitted for approval.

## Approver logic

This contract registers a hook with the approval contract to automatically create proposals in the approver DAO. When this contract recieves a prop as hook from `cwd-pre-propose-approval-flow`, it makes an approval prop. If approved, the approval prop calls the approve message on this contract. When prop fails it fires off reject call.

## Deposits

This contract does not handle deposit logic, as it works in conjunction with the `cwd-pre-propose-approval-flow` contract which handles to the deposits.
