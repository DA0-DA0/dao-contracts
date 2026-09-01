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
therefore dispatch each hook with a producer-owned reply ID and reply on both
success and failure. Producers must handle that reply ID (see
`stake_hook_reply_response`), which records a failed hook call as attributes on
the producer's transaction (`action: stake_hook_failed`, `hook: stake|unstake`,
`error: <receiver error>`) while leaving the hook registered and the staking
transaction successful.

### Vote Hooks
Vote hooks are fired when new votes are cast.

You can read more about vote hooks in our [wiki](https://github.com/DA0-DA0/dao-contracts/wiki/Proposal-Hooks-Interactions).
