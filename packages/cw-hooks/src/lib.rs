#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"))]

use thiserror::Error;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, CustomQuery, Deps, StdError, StdResult, Storage, SubMsg};
use cw_storage_plus::Item;

#[cw_serde]
pub struct HooksResponse {
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

// store all hook addresses in one item. We cannot have many of them before the contract becomes unusable anyway.
pub struct Hooks<'a>(Item<'a, Vec<Addr>>);

impl<'a> Hooks<'a> {
    pub const fn new(storage_key: &'a str) -> Self {
        Hooks(Item::new(storage_key))
    }

    pub fn add_hook(&self, storage: &mut dyn Storage, addr: Addr) -> Result<(), HookError> {
        let mut hooks = self.0.may_load(storage)?.unwrap_or_default();
        if !hooks.iter().any(|h| h == &addr) {
            hooks.push(addr);
        } else {
            return Err(HookError::HookAlreadyRegistered {});
        }
        Ok(self.0.save(storage, &hooks)?)
    }

    pub fn remove_hook(&self, storage: &mut dyn Storage, addr: Addr) -> Result<(), HookError> {
        let mut hooks = self.0.load(storage)?;
        if let Some(p) = hooks.iter().position(|x| x == &addr) {
            hooks.remove(p);
        } else {
            return Err(HookError::HookNotRegistered {});
        }
        Ok(self.0.save(storage, &hooks)?)
    }

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

    pub fn hook_count(&self, storage: &dyn Storage) -> StdResult<u32> {
        // The WASM VM (as of version 1) is 32 bit and sets limits for
        // memory accordingly:
        // <https://webassembly.github.io/spec/core/syntax/types.html#syntax-limits>. We
        // can safely return a u32 here as that's the biggest size in
        // the WASM VM.
        Ok(self.0.may_load(storage)?.unwrap_or_default().len() as u32)
    }

    pub fn query_hooks<Q: CustomQuery>(&self, deps: Deps<Q>) -> StdResult<HooksResponse> {
        let hooks = self.0.may_load(deps.storage)?.unwrap_or_default();
        let hooks = hooks.into_iter().map(String::from).collect();
        Ok(HooksResponse { hooks })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::{coins, testing::mock_dependencies, BankMsg};

    // Shorthand for an unchecked address.
    macro_rules! addr {
        ($x:expr ) => {
            Addr::unchecked($x)
        };
    }

    #[test]
    fn test_hooks() {
        let mut deps = mock_dependencies();
        let storage = &mut deps.storage;
        let hooks = Hooks::new("hooks");
        hooks.add_hook(storage, addr!("ekez")).unwrap();
        hooks.add_hook(storage, addr!("meow")).unwrap();

        assert_eq!(hooks.hook_count(storage).unwrap(), 2);

        hooks.remove_hook_by_index(storage, 0).unwrap();

        assert_eq!(hooks.hook_count(storage).unwrap(), 1);

        let msgs = hooks
            .prepare_hooks(storage, |a| {
                Ok(SubMsg::reply_always(
                    BankMsg::Burn {
                        amount: coins(a.as_str().len() as u128, "uekez"),
                    },
                    2,
                ))
            })
            .unwrap();

        assert_eq!(
            msgs,
            vec![SubMsg::reply_always(
                BankMsg::Burn {
                    amount: coins(4, "uekez"),
                },
                2,
            )]
        );

        let HooksResponse { hooks: the_hooks } = hooks.query_hooks(deps.as_ref()).unwrap();

        assert_eq!(the_hooks, vec![addr!("meow")]);
    }
}
