# Proposal Approver Contract

[![dao-pre-propose-approver on crates.io](https://img.shields.io/crates/v/dao-pre-propose-approver.svg?logo=rust)](https://crates.io/crates/dao-pre-propose-approver)
[![docs.rs](https://img.shields.io/docsrs/dao-pre-propose-approver?logo=docsdotrs)](https://docs.rs/dao-pre-propose-approver/latest/dao_pre_propose_approver/)

This contract works in conjuction with `cwd-pre-propose-approval-single` and allows for automatically creating approval proposals when a proposal is submitted for approval.

## Approver Logic

On instantiation, this contract registers a hook with the approval contract to automatically create proposals in the approver DAO.

When this contract recieves a proposal as hook from `cwd-pre-propose-approval-single`, it makes an approval propose in the approval DAO. If approved, the approval proposal calls the approve message on this contract when executed. If the proposal is rejected and closed it fires off reject call.

```text
┌──────────┐         Approver DAO Registers Prop Submission Hook
│          │       ┌──────────────────────────────────────────────┐
│  Account │       │                                              │
│          │       │                                              │
└─────┬────┘       │    Prop Submission Hook creates              │
      │            │    new prop in Approver DAO                  │
      │ Makes prop │   ┌───────────────────────────┐              │
      ▼            ▼   │                           ▼              │
┌──────────────────────┴─┐             ┌────────────────────────┐ │
│                        │             │                        │ │
│  Pre-propose Approval  │             │  Pre-propose Approver  │ │
│                        │◄──┐         │                        │ │
└───────────┬────────────┘   │         └───────────┬────────────┘ │
            │                │                     │              │
            │ Creates prop   │                     │ Creates      │
            │ on approval    │                     │ prop         │
            ▼                │                     ▼              │
┌────────────────────────┐   │         ┌────────────────────────┐ │
│                        │   │         │                        │ │
│     Proposal Single    │   │         │     Proposal Single    │ │
│                        │   │         │                        │ │
└───────────┬────────────┘   │         └───────────┬────────────┘ │
            │                │ Approver            │              │
            │ Normal voting  │ Approves            │ Voting       │
            │                │ or                  │              │
            ▼                │ Rejects             ▼              │
┌────────────────────────┐   │         ┌────────────────────────┐ │
│                        │   │         │                        │ │
│       Main DAO         │   └─────────┤     Approver DAO       ├─┘
│                        │             │                        │
└────────────────────────┘             └────────────────────────┘
```

## Deposits

This contract does not handle deposits. It works in conjunction with the `cwd-pre-propose-approval-single` contract, which handles the proposal deposits.

### Resources

More about the [pre-propose design](https://github.com/DA0-DA0/dao-contracts/wiki/Pre-propose-module-design).

More about [pre-propose modules](https://github.com/DA0-DA0/dao-contracts/wiki/DAO-DAO-Contracts-Design#pre-propose-modules).
