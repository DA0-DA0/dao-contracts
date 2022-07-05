use cosmwasm_std::{Addr, Uint128};
use cw721_controllers::NftClaims;
use cw_controllers::Hooks;
use cw_storage_plus::{Item, SnapshotItem, SnapshotMap, Strategy};
use cw_utils::Duration;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Config {
    pub owner: Option<Addr>,
    pub manager: Option<Addr>,
    pub nft_address: Addr,
    pub unstaking_duration: Option<Duration>,
}

pub const CONFIG: Item<Config> = Item::new("config");

/// Maps addresses to the set of NFTs they have staked with this
/// contract at a given height.
///
/// NOTE: we can get the length of this set in constant time as:
/// - HashSet uses hashbrown::HashMap.
/// - hashbrown::HashMap uses hashbrown::RawTable.
/// - hashbrown::RawTable len calls are
///   [constant time](https://docs.rs/hashbrown/latest/src/hashbrown/raw/mod.rs.html#921-923).
pub const STAKED_NFTS_PER_OWNER: SnapshotMap<&Addr, HashSet<String>> = SnapshotMap::new(
    "staked_nfts_per_owner",
    "staked_nfts_per_owner__checkpoints",
    "staked_nfts_per_owner__changelog",
    Strategy::EveryBlock,
);

/// The number of NFTs staked with this contract at a given height.
pub const TOTAL_STAKED_NFTS: SnapshotItem<Uint128> = SnapshotItem::new(
    "total_staked_nfts",
    "total_staked_nfts__checkpoints",
    "total_staked_nfts__changelog",
    Strategy::EveryBlock,
);

/// The maximum number of claims that may be outstanding.
pub const MAX_CLAIMS: u64 = 100;
pub const NFT_CLAIMS: NftClaims = NftClaims::new("nft_claims");

// Hooks to contracts that will receive staking and unstaking messages
pub const HOOKS: Hooks = Hooks::new("hooks");
