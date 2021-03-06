use cosmwasm_std::{Addr, Order, StdError, StdResult, Storage};
use cw_storage_plus::{Item, Map};

pub const OWNER: Item<Addr> = Item::new("owner");
pub const GROUPS: Groups = Groups::new("groups", "addresses", "group_names");

pub struct Groups<'a> {
    pub groups_to_addresses: Map<'a, (&'a str, &'a Addr), bool>,
    pub addresses_to_groups: Map<'a, (&'a Addr, &'a str), bool>,
    pub group_names: Map<'a, &'a str, bool>, // This is maintained so that you can have groups with no addresses in them.
}

impl<'a> Groups<'a> {
    pub const fn new(
        groups_storage_key: &'a str,
        addresses_storage_key: &'a str,
        group_names_storage_key: &'a str,
    ) -> Self {
        Groups {
            groups_to_addresses: Map::new(groups_storage_key),
            addresses_to_groups: Map::new(addresses_storage_key),
            group_names: Map::new(group_names_storage_key),
        }
    }

    pub fn update(
        &self,
        storage: &mut dyn Storage,
        name: &'a str,
        addresses_to_add: Option<Vec<Addr>>,
        addresses_to_remove: Option<Vec<Addr>>,
    ) -> StdResult<()> {
        if let Some(addrs) = addresses_to_add {
            for addr in addrs {
                self.groups_to_addresses
                    .save(storage, (name, &addr), &true)?;
                self.addresses_to_groups
                    .save(storage, (&addr, name), &true)?;
            }
        }

        if let Some(addrs) = addresses_to_remove {
            for addr in addrs {
                self.groups_to_addresses.remove(storage, (name, &addr));
                self.addresses_to_groups.remove(storage, (&addr, name));
            }
        }

        // Update group name
        self.group_names.save(storage, name, &true)?;

        Ok(())
    }

    pub fn remove_group(&self, storage: &mut dyn Storage, name: &'a str) -> StdResult<()> {
        let mut to_remove: Vec<(&str, Addr)> = Vec::new();
        let prefix = self.groups_to_addresses.prefix(name);
        let mut keys = prefix
            .keys(storage, None, None, Order::Ascending)
            .peekable();

        if keys.peek().is_none() {
            return Err(StdError::not_found("group"));
        }

        // Retrieve all addresses from group map.
        keys.into_iter()
            .try_for_each::<_, StdResult<()>>(|element| {
                // Collect all (group, address) tuples to be removed.
                let addr = element?;
                to_remove.push((name, addr));
                Ok(())
            })?;

        // Remove each group-address pair.
        for element in to_remove {
            self.groups_to_addresses
                .remove(storage, (element.0, &element.1));
            self.addresses_to_groups
                .remove(storage, (&element.1, element.0));
        }

        // Remove group name.
        self.group_names.remove(storage, name);

        Ok(())
    }

    pub fn list_addresses(&self, storage: &dyn Storage, group: String) -> StdResult<Vec<Addr>> {
        // Retrieve all addresses under this group, returning error if group not found.
        if !self.group_names.has(storage, &group) {
            return Err(StdError::NotFound {
                kind: "group".to_string(),
            });
        }

        let addresses = GROUPS
            .groups_to_addresses
            .prefix(&group)
            .keys(storage, None, None, Order::Ascending)
            .collect::<StdResult<Vec<Addr>>>()
            .map_err(|_| StdError::not_found("group"))?;

        Ok(addresses)
    }

    pub fn list_groups(&self, storage: &dyn Storage, addr: &Addr) -> Vec<String> {
        // Return groups, or an empty vec if failed to load (address probably doesn't exist).
        // It doesn't make sense to ask for the addresses in a group if the group doesn't exist, which is why
        // we return an error in query_list_addresses; however, here in query_list_groups, it makes sense
        // to return an empty list when an address is not in any groups since this is a valid case.
        GROUPS
            .addresses_to_groups
            .prefix(addr)
            .keys(storage, None, None, Order::Ascending)
            .collect::<StdResult<Vec<String>>>()
            .unwrap_or_default()
    }

    pub fn is_in_group(
        &self,
        storage: &dyn Storage,
        addr: &Addr,
        group: String,
    ) -> StdResult<bool> {
        if !self.group_names.has(storage, &group) {
            return Err(StdError::NotFound {
                kind: "group".to_string(),
            });
        }

        Ok(GROUPS
            .groups_to_addresses
            .load(storage, (&group, addr))
            .is_ok())
    }
}
