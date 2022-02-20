use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Empty, Querier, QuerierWrapper, QueryRequest, StdResult, Storage, WasmQuery};
use cw_storage_plus::Item;

pub const CONTRACT: Item<ContractVersion> = Item::new("contract_info");

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct ContractVersion {
    /// contract is the crate name of the implementing contract, eg. `crate:cw20-base`
    /// we will use other prefixes for other languages, and their standard global namespacing
    pub contract: String,
    /// version is any string that this implementation knows. It may be simple counter "1", "2".
    /// or semantic version on release tags "v0.7.0", or some custom feature flag list.
    /// the only code that needs to understand the version parsing is code that knows how to
    /// migrate from the given contract (and is tied to it's implementation somehow)
    pub version: String,

    // Schema Query (two options, 1 to have this as optional or second to require it). by requiring it we enable this to be indexable
    pub query_ipfs: String,
}

/// get_contract_version can be use in migrate to read the previous version of this contract
pub fn get_contract_version(store: &dyn Storage) -> StdResult<ContractVersion> {
    CONTRACT.load(store)
}

/// set_contract_version should be used in instantiate to store the original version, and after a successful
/// migrate to update it
pub fn set_contract_version<T: Into<String>, U: Into<String>, V: Into<String>>(
    store: &mut dyn Storage,
    name: T,
    version: U,
    query_ipfs: V
) -> StdResult<()> {
    let val = ContractVersion {
        contract: name.into(),
        version: version.into(),
        query_ipfs: query_ipfs.into()
    };
    CONTRACT.save(store, &val)
}

/// This will make a raw_query to another contract to determine the current version it
/// claims to be. This should not be trusted, but could be used as a quick filter
/// if the other contract exists and claims to be a cw20-base contract for example.
/// (Note: you usually want to require *interfaces* not *implementations* of the
/// contracts you compose with, so be careful of overuse)
pub fn query_contract_info<Q: Querier, T: Into<String>>(
    querier: &Q,
    contract_addr: T,
) -> StdResult<ContractVersion> {
    let req = QueryRequest::Wasm(WasmQuery::Raw {
        contract_addr: contract_addr.into(),
        key: CONTRACT.as_slice().into(),
    });
    QuerierWrapper::<Empty>::new(querier).query(&req)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::MockStorage;

    #[test]
    fn get_and_set_work() {
        let mut store = MockStorage::new();

        // error if not set
        assert!(get_contract_version(&store).is_err());

        // set and get
        let contract_name = "crate:cw20-base";
        let contract_version = "0.2.0";
        let contract_query_ipfs = "https://gateway.pinata.cloud/ipfs/QmWac4BCRNMEZyqr5xuC7GmQp3kbeip2MxS9zj2QDuG2vz";
        set_contract_version(&mut store, contract_name, contract_version, contract_query_ipfs).unwrap();

        let loaded = get_contract_version(&store).unwrap();
        let expected = ContractVersion {
            contract: contract_name.to_string(),
            version: contract_version.to_string(),
            query_ipfs: contract_query_ipfs.to_string()
        };
        assert_eq!(expected, loaded);
    }
}
