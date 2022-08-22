use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use cosmwasm_std::{Addr, CustomQuery, Deps, StdError, StdResult, Storage, SubMsg};
use cw_storage_plus::Item;

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, JsonSchema, Debug)]
pub struct HooksResponse {
    /// A list of addresses that are registered to be receiving hooks.
    pub hooks: Vec<String>,
}

#[derive(Error, Debug, PartialEq)]
pub enum HookError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Given address already registered as a hook")]
    HookAlreadyRegistered {},

    #[error("Given address not registered as a hook")]
    HookNotRegistered {},
}

/// We store all hook addresses in one item. If contracts could
/// support large numbers of hooks storage reads could get expensive
/// here. Broadly speaking though, a contract will run out of gas
/// firing hooks before this load becomes meaningfully expensive.
pub struct Hooks<'a>(Item<'a, Vec<Addr>>);

impl<'a> Hooks<'a> {
    /// Creates a new set of hooks with the specified
    /// STORAGE_KEY. Hooks will be stored at this location.
    pub const fn new(storage_key: &'a str) -> Self {
        Hooks(Item::new(storage_key))
    }

    /// Adds a new hook. The hook must not already be registered.
    pub fn add_hook(&self, storage: &mut dyn Storage, addr: Addr) -> Result<(), HookError> {
        let mut hooks = self.0.may_load(storage)?.unwrap_or_default();
        if !hooks.iter().any(|h| h == &addr) {
            hooks.push(addr);
        } else {
            return Err(HookError::HookAlreadyRegistered {});
        }
        Ok(self.0.save(storage, &hooks)?)
    }

    /// Removes a hook. The hook must have been previously added.
    pub fn remove_hook(&self, storage: &mut dyn Storage, addr: Addr) -> Result<(), HookError> {
        let mut hooks = self.0.load(storage)?;
        if let Some(p) = hooks.iter().position(|x| x == &addr) {
            hooks.remove(p);
        } else {
            return Err(HookError::HookNotRegistered {});
        }
        Ok(self.0.save(storage, &hooks)?)
    }

    /// Removes a hook by index it's index. Panics if the index is out
    /// of bounds.
    ///
    /// This is used by the `proposal-hooks` and `vote-hooks` packages
    /// in order to remove hooks if they fail to execute. See the
    /// `reply` method of `cw-dao-proposal-single` and those packages
    /// for more information.
    pub fn remove_hook_by_index(
        &self,
        storage: &mut dyn Storage,
        index: u64,
    ) -> Result<Addr, HookError> {
        let mut hooks = self.0.load(storage)?;
        let hook = hooks.remove(index as usize);
        self.0.save(storage, &hooks)?;
        Ok(hook)
    }

    /// Applies a method to the list of hooks which transforms them
    /// into submessages.
    pub fn prepare_hooks<F: FnMut(Addr) -> StdResult<SubMsg>>(
        &self,
        storage: &dyn Storage,
        prep: F,
    ) -> StdResult<Vec<SubMsg>> {
        self.0
            .may_load(storage)?
            .unwrap_or_default()
            .into_iter()
            .map(prep)
            .collect()
    }

    /// Gets the hooks currently registered. Intended to be use in the
    /// query methods for a contract.
    pub fn query_hooks<Q: CustomQuery>(&self, deps: Deps<Q>) -> StdResult<HooksResponse> {
        let hooks = self.0.may_load(deps.storage)?.unwrap_or_default();
        let hooks = hooks.into_iter().map(String::from).collect();
        Ok(HooksResponse { hooks })
    }
}
