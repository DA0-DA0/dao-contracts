# Proposal Approver Contract

This contract works in conjuction with `cwd-pre-propose-approval-single` and allows for automatically creating approval proposals when a proposal is submitted for approval.

## Approver Logic

On instantiation, this contract registers a hook with the approval contract to automatically create proposals in the approver DAO.

When this contract recieves a prop as hook from `cwd-pre-propose-approval-single`, it makes an approval prop. If approved, the approval prop calls the approve message on this contract when executed. When prop is rejected and closed it fires off reject call.

``` 
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

This contract does not handle deposit logic, as it works in conjunction with the `cwd-pre-propose-approval-single` contract which handles to the deposits.
