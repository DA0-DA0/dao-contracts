use cosmwasm_std::{Addr, Deps, StdResult};

/// validate addresses and add to and/or remove from an existing list of
/// addresses, removing any duplicates. mutates the original list.
pub fn add_and_remove_addresses(
    deps: Deps,
    list: &mut Vec<Addr>,
    add: Option<Vec<String>>,
    remove: Option<Vec<String>>,
) -> StdResult<()> {
    if let Some(add) = add {
        let mut addrs = add
            .iter()
            .map(|addr| deps.api.addr_validate(addr))
            .collect::<StdResult<Vec<Addr>>>()?;

        list.append(&mut addrs);
        list.sort();
        list.dedup();
    }

    if let Some(remove) = remove {
        let addrs = remove
            .iter()
            .map(|addr| deps.api.addr_validate(addr))
            .collect::<StdResult<Vec<Addr>>>()?;

        list.retain(|a| !addrs.contains(a));
    }

    Ok(())
}
