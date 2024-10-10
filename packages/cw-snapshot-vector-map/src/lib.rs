#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"))]

use cw20::Expiration;
use cw_utils::Duration;
use serde::de::DeserializeOwned;
use serde::Serialize;

use cosmwasm_std::{BlockInfo, StdResult, Storage};
use cw_storage_plus::{KeyDeserialize, Map, Prefixer, PrimaryKey, SnapshotMap, Strategy};

/// Map to a vector that allows reading the subset of items that existed at a
/// specific height in the past based on when items were added, removed, and
/// expired.
pub struct SnapshotVectorMap<'a, K, V> {
    /// All items for a key, indexed by ID.
    items: Map<'a, &'a (K, u64), V>,
    /// The next item ID to use per-key.
    next_ids: Map<'a, K, u64>,
    /// The IDs of the items that are active for a key at a given height, and
    /// optionally when they expire.
    active: SnapshotMap<'a, K, Vec<(u64, Option<Expiration>)>>,
}

/// A loaded item from the vector, including its ID and expiration.
#[derive(Debug, Clone, PartialEq)]
pub struct LoadedItem<V> {
    /// The ID of the item within the vector, which can be used to update or
    /// remove it.
    pub id: u64,
    /// The item.
    pub item: V,
    /// When the item expires, if set.
    pub expiration: Option<Expiration>,
}

impl<'a, K, V> SnapshotVectorMap<'a, K, V> {
    /// Creates a new [`SnapshotVectorMap`] with the given storage keys.
    ///
    /// Example:
    ///
    /// ```rust
    /// use cw_snapshot_vector_map::SnapshotVectorMap;
    ///
    /// SnapshotVectorMap::<&[u8], &str>::new(
    ///     "data__items",
    ///     "data__next_ids",
    ///     "data__active",
    ///     "data__active__checkpoints",
    ///     "data__active__changelog",
    /// );
    /// ```
    pub const fn new(
        items_key: &'static str,
        next_ids_key: &'static str,
        active_key: &'static str,
        active_checkpoints_key: &'static str,
        active_changelog_key: &'static str,
    ) -> Self {
        SnapshotVectorMap {
            items: Map::new(items_key),
            next_ids: Map::new(next_ids_key),
            active: SnapshotMap::new(
                active_key,
                active_checkpoints_key,
                active_changelog_key,
                Strategy::EveryBlock,
            ),
        }
    }
}

impl<'a, K, V> SnapshotVectorMap<'a, K, V>
where
    // values can be serialized and deserialized
    V: Serialize + DeserializeOwned,
    // keys can be primary keys, cloned, deserialized, and prefixed
    K: Clone + KeyDeserialize + Prefixer<'a> + PrimaryKey<'a>,
    // &(key, ID) is a key in a map
    for<'b> &'b (K, u64): PrimaryKey<'b>,
{
    /// Adds an item to the vector at the current block, optionally expiring in
    /// the future, returning the ID of the new item. This block should be
    /// greater than or equal to the blocks all previous items were
    /// added/removed at. Pushing to the past will lead to incorrect behavior.
    pub fn push(
        &self,
        store: &mut dyn Storage,
        k: &K,
        data: &V,
        block: &BlockInfo,
        expire_in: Option<Duration>,
    ) -> StdResult<u64> {
        // get next ID for the key, defaulting to 0
        let next_id = self
            .next_ids
            .may_load(store, k.clone())?
            .unwrap_or_default();

        // add item to the list of all items for the key
        self.items.save(store, &(k.clone(), next_id), data)?;

        // get active list for the key
        let mut active = self.active.may_load(store, k.clone())?.unwrap_or_default();

        // remove expired items
        active.retain(|(_, expiration)| {
            expiration.map_or(true, |expiration| !expiration.is_expired(block))
        });

        // add new item and save list
        active.push((next_id, expire_in.map(|d| d.after(block))));

        // save the new list
        self.active.save(store, k.clone(), &active, block.height)?;

        // update next ID
        self.next_ids.save(store, k.clone(), &(next_id + 1))?;

        Ok(next_id)
    }

    /// Removes an item from the vector by ID and returns it. The block should
    /// be greater than or equal to the blocks all previous items were
    /// added/removed at. Removing from the past will lead to incorrect
    /// behavior.
    pub fn remove(
        &self,
        store: &mut dyn Storage,
        k: &K,
        id: u64,
        block: &BlockInfo,
    ) -> StdResult<V> {
        // get active list for the key
        let mut active = self.active.may_load(store, k.clone())?.unwrap_or_default();

        // remove item and any expired items
        active.retain(|(active_id, expiration)| {
            active_id != &id && expiration.map_or(true, |expiration| !expiration.is_expired(block))
        });

        // save the new list
        self.active.save(store, k.clone(), &active, block.height)?;

        // load and return the item
        self.load_item(store, k, id)
    }

    /// Loads paged items at the given block that are not expired.
    pub fn load(
        &self,
        store: &dyn Storage,
        k: &K,
        block: &BlockInfo,
        limit: Option<u64>,
        offset: Option<u64>,
    ) -> StdResult<Vec<LoadedItem<V>>> {
        let offset = offset.unwrap_or_default() as usize;
        let limit = limit.unwrap_or(u64::MAX) as usize;

        let active_ids = self
            .active
            .may_load_at_height(store, k.clone(), block.height)?
            .unwrap_or_default();

        // load paged items, skipping expired ones
        let items = active_ids
            .iter()
            .filter(|(_, expiration)| expiration.map_or(true, |exp| !exp.is_expired(block)))
            .skip(offset)
            .take(limit)
            .map(|(id, expiration)| -> StdResult<LoadedItem<V>> {
                let item = self.load_item(store, k, *id)?;
                Ok(LoadedItem {
                    id: *id,
                    item,
                    expiration: *expiration,
                })
            })
            .collect::<StdResult<Vec<_>>>()?;

        Ok(items)
    }

    /// Loads all items at the given block that are not expired.
    pub fn load_all(
        &self,
        store: &dyn Storage,
        k: &K,
        block: &BlockInfo,
    ) -> StdResult<Vec<LoadedItem<V>>> {
        self.load(store, k, block, None, None)
    }

    /// Loads an item from the vector by ID.
    pub fn load_item(&self, store: &dyn Storage, k: &K, id: u64) -> StdResult<V> {
        let item = self.items.load(store, &(k.clone(), id))?;
        Ok(item)
    }

    /// Loads an item from the vector by ID, if it exists.
    pub fn may_load_item(&self, store: &dyn Storage, k: &K, id: u64) -> StdResult<Option<V>> {
        self.items.may_load(store, &(k.clone(), id))
    }
}

#[cfg(test)]
mod tests;
