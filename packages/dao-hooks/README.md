# DAO Hooks
This package provides an interface for managing and dispatching proposal, 
staking, and voting related hooks. 

### NFT Stake Hooks
Staking hooks are fired when NFTs are staked or unstaked in a DAO.

NFT stake hooks follow the same failure handling as [stake hooks](#stake-hooks)
below.

### Proposal Hooks
There are two types of proposal hooks:
- **New Proposal Hook:** fired when a new proposal is created.
- **Proposal Staus Changed Hook:** fired when a proposal's status changes. 

Our wiki contains more info on [Proposal Hooks](https://github.com/DA0-DA0/dao-contracts/wiki/Proposal-Hooks-Interactions).

### Stake Hooks
Staking hooks are fired when tokens are staked or unstaked in a DAO.

A hook receiver must never be able to block staking or unstaking, and a hook
that fails must not be silently removed. The stake and NFT stake hook helpers
therefore dispatch every hook with `reply_always`, tagging each submessage with
`STAKE_HOOK_REPLY_ID_BASE`/`UNSTAKE_HOOK_REPLY_ID_BASE` plus the hook's index in
the producer's registry. Producers must call `handle_stake_hook_reply` from
their `reply` entry point and must not use reply IDs inside either range.

A failed hook leaves the receiver registered, lets the staking transaction
succeed, and records the failure as attributes on that transaction:
`action: stake_hook_failed`, `hook: stake|unstake`, `addr: <receiver>` and
`error: <error>`. CosmWasm chains redact submessage errors before they reach a
reply handler, so on chain `error` is a codespace and code rather than the
receiver's own message. `addr` is the identifier a DAO needs in order to act on
the failure, which is why the reply ID carries the hook's index.

### Vote Hooks
Vote hooks are fired when new votes are cast.

You can read more about vote hooks in our [wiki](https://github.com/DA0-DA0/dao-contracts/wiki/Proposal-Hooks-Interactions).
