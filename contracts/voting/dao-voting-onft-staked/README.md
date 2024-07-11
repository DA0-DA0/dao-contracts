# `dao-voting-onft-staked`

This is a basic implementation of an NFT staking contract that supports
OmniFlix's NFT standard:
[x/onft](https://github.com/OmniFlix/omniflixhub/tree/main/x/onft).

Staked tokens can be unbonded with a configurable unbonding period. Staked balances can be queried at any arbitrary height by external contracts. This contract implements the interface needed to be a DAO DAO [voting module](https://github.com/DA0-DA0/dao-contracts/wiki/DAO-DAO-Contracts-Design#the-voting-module).

### Stake process

Unlike the base cw721 smart contract, the x/onft SDK module doesn't support
executing a smart contract on NFT transfer, so the stake process is broken up
into three steps:

1. The sender calls `PrepareStake` to inform this staking contract of the NFTs
   that are about to be staked. This will succeed only if the sender currently
   owns the NFT(s).
2. The sender then transfers the NFT(s) to the staking contract.
3. The sender calls `ConfirmStake` on this staking contract which confirms the
   NFTs were transferred to it and registers the stake.

In case this process is interrupted, or executed incorrectly (e.g. the sender
accidentally transfers an NFT to the staking contract without first preparing
it), there is also a `CancelStake` action to help recover NFTs. If called by:

- the original stake preparer, the preparation will be canceled, and the NFT(s)
  will be sent back if the staking contract owns them.
- the current NFT(s) owner, the preparation will be canceled, if any.
- the DAO, the preparation will be canceled (if any exists), and the NFT(s) will
  be sent to the specified recipient (if the staking contract owns them). if no
  recipient is specified but the NFT was prepared, it will be sent back to the
  preparer.

The recipient field only applies when the sender is the DAO. In the other cases,
the NFT(s) will always be sent back to the sender. Note: if the NFTs were sent
to the staking contract, but no stake was prepared, only the DAO will be able to
correct this and send them somewhere.

The `PrepareStake` step overrides any previous `PrepareStake` calls as long as
the new sender owns the NFT(s) and the first stake was never confirmed (which
should be impossible if someone else now owns the NFT(s)). Thus there is no
combination of messages or steps where someone can stake nor prevent stake when
it would otherwise be valid. A stake is only ever confirmed if it was prepared
and transferred by the same address confirming, and the DAO can always recover
an NFT that accidentally skipped the preparation step.
