use cosmwasm_std::Addr;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    /// The admin has the ability to add new items to the address list
    /// and to update the admin.
    pub admin: Addr,
}

/// An address and its priority. Items are ordered first by their
/// priority and then alphabetically by their address.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub struct AddressItem {
    pub priority: u32,
    pub addr: Addr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// Updates the contents of the address list.
    UpdateAddresses {
        to_add: Vec<AddressItem>,
        to_remove: Vec<AddressItem>,
    },
    /// Updates the admin of the contract.
    UpdateAdmin { new_admin: Addr },
}

/// Query message types for the contract. Note: the webassembly
/// virtual machine is 32 bits so indexes are specified as u32 values.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Gets the address list from the contract. Items are returned in
    /// order of their priority. Items with higher priorities are
    /// returned first. Returns Vec<AddressItem>.
    GetAddresses {
        /// The lowest priority to include in results. Inclusive.
        start: Option<u32>,
        /// The highest priority to include in results. Exclusive.
        end: Option<u32>,
    },
    /// Checks if `addr` is contained in the contract's address
    /// list. Returns bool.
    CheckAddress { addr: Addr },
    /// Gets the admin of the contract. Returns Addr.
    GetAdmin {},
    /// Gets the number of addresses in the contract. Returns usize
    /// (u32 in webassembly).
    GetAddressCount {},
}
