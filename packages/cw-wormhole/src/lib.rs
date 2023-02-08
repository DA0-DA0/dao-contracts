use std::{
    marker::PhantomData,
    ops::{Add, Sub},
};

use serde::de::DeserializeOwned;
use serde::Serialize;

use cosmwasm_std::{Order, StdResult, Storage};
use cw_storage_plus::{Bound, KeyDeserialize, Map, PrimaryKey};

/// A map that ensures that the gas cost of updating a value is higher
/// than the cost of loading a value and allows updating values in the
/// future. The cost of loading a value from this map is O(1) in gas.
///
/// This map has a special high-performance case if it is being used
/// to track unbonding tokens. In that case, the runtime to update a
/// key is O(# times unbonding duration has changed). For a proof of
/// this, and further runtime analysis see [this
/// essay](https://gist.github.com/0xekez/15fab6436ed593cbd59f0bdf7ecf1f61).
///
/// # Example
///
/// ```
/// # use cosmwasm_std::{testing::mock_dependencies, Uint128};
/// # use cw_wormhole::Wormhole;
/// let storage = &mut mock_dependencies().storage;
/// let fm: Wormhole<String, Uint128> = Wormhole::new("ns");
///
/// fm.increment(storage, "fm".to_string(), 10, Uint128::new(1))
///     .unwrap();
/// fm.increment(storage, "fm".to_string(), 9, Uint128::new(2))
///     .unwrap();
///
/// // no value exists at time=8
/// assert_eq!(fm.load(storage, "fm".to_string(), 8).unwrap(), None);
/// // value was incremented by 2 at time=9
/// assert_eq!(
///     fm.load(storage, "fm".to_string(), 9).unwrap(),
///     Some(Uint128::new(2))
/// );
/// // value was incremented by 1 at time=10 making final value 3
/// assert_eq!(
///     fm.load(storage, "fm".to_string(), 10).unwrap(),
///     Some(Uint128::new(3))
/// );
/// ```
pub struct Wormhole<'n, K, V> {
    namespace: &'n str,
    k: PhantomData<K>,
    v: PhantomData<V>,
}

impl<'n, K, V> Wormhole<'n, K, V> {
    /// Creates a new map using the provided namespace.
    ///
    /// The namespace identifies the prefix in the SDK's prefix
    /// store that values and keys will be stored under.
    ///
    /// # Example
    ///
    /// ```
    /// # use cw_wormhole::Wormhole;
    /// # use cosmwasm_std::{Addr, Uint128};
    ///
    /// pub const MAP: Wormhole<&Addr, Uint128> = Wormhole::new("unbonded_balances");
    /// ```
    pub const fn new(namespace: &'n str) -> Self {
        Self {
            namespace,
            k: PhantomData,
            v: PhantomData,
        }
    }
}

impl<'n, K, V> Wormhole<'n, K, V>
where
    // 1. values in the map can be serialized and deserialized
    V: Serialize + DeserializeOwned + Default + Clone,
    // 1.1. keys in the map can be cloned
    K: Clone,
    // 2. &(key, time) is a value key in a map
    for<'a> &'a (K, u64): PrimaryKey<'a>,
    // 3. the suffix of (2) is a valid key and constructable from a
    //    time (u64)
    for<'a> <&'a (K, u64) as PrimaryKey<'a>>::Suffix: PrimaryKey<'a> + From<u64>,
    // 4. K can be converted into the prefix of (2)
    for<'a> K: Into<<&'a (K, u64) as PrimaryKey<'a>>::Prefix>,
    // 5. when deserializing a key the result has a static lifetime
    //    and can be converted into a key. required by the `range`
    //    call in the `load` method
    for<'a> <<&'a (K, u64) as PrimaryKey<'a>>::Suffix as KeyDeserialize>::Output:
        'static + Into<u64>,
{
    /// Loads the value at a key at the specified time. If the key has
    /// no value at that time, returns `None`. Returns `Some(value)`
    /// otherwise.
    pub fn load(&self, storage: &dyn Storage, k: K, t: u64) -> StdResult<Option<V>> {
        let now = Bound::inclusive(t);
        Ok(self
            .snapshots()
            .prefix(k.into())
            .range(storage, None, Some(now), Order::Descending)
            .next()
            .transpose()?
            .map(|(_k, v)| v))
    }

    pub fn increment(&self, storage: &mut dyn Storage, k: K, t: u64, i: V) -> StdResult<()>
    where
        V: Add<Output = V>,
    {
        self.update(storage, k, t, &|v| v + i.clone())
    }

    pub fn decrement(&self, storage: &mut dyn Storage, k: K, t: u64, i: V) -> StdResult<()>
    where
        V: Sub<Output = V>,
    {
        self.update(storage, k, t, &|v| v - i.clone())
    }

    const fn snapshots<'a>(&self) -> Map<'n, &'a (K, u64), V> {
        Map::new(self.namespace)
    }

    fn update(
        &self,
        storage: &mut dyn Storage,
        k: K,
        t: u64,
        update: &dyn Fn(V) -> V,
    ) -> StdResult<()> {
        for (t, v) in self
            .snapshots()
            .prefix(k.clone().into())
            .range(storage, Some(Bound::exclusive(t)), None, Order::Ascending)
            .collect::<StdResult<Vec<_>>>()?
            .into_iter()
        {
            self.snapshots()
                .save(storage, &(k.clone(), t.into()), &update(v))?;
        }
        let v = self.load(storage, k.clone(), t)?.unwrap_or_default();
        self.snapshots().save(storage, &(k, t), &update(v))
    }
}

#[cfg(test)]
mod tests;
