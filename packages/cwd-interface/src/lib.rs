#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"))]

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Binary, CosmosMsg, Empty, WasmMsg};

pub mod voting;

/// The cw-core interface.
#[cw_serde]
pub enum ExecuteMsg {
    /// Causes the core module to execute all of MSGS in order. Only
    /// callabale by a proposal module.1
    ExecuteProposalHook { msgs: Vec<CosmosMsg<Empty>> },
}

/// Information about the CosmWasm level admin of a contract. Used in
/// conjunction with `ModuleInstantiateInfo` to instantiate modules.
#[cw_serde]
pub enum Admin {
    /// Set the admin to a specified address.
    Address { addr: String },
    /// Sets the admin as the core module address.
    CoreModule {},
}

/// Information needed to instantiate a module.
#[cw_serde]
pub struct ModuleInstantiateInfo {
    /// Code ID of the contract to be instantiated.
    pub code_id: u64,
    /// Instantiate message to be used to create the contract.
    pub msg: Binary,
    /// CosmWasm level admin of the instantiated contract. See:
    /// <https://docs.cosmwasm.com/docs/1.0/smart-contracts/migration>
    pub admin: Option<Admin>,
    /// Label for the instantiated contract.
    pub label: String,
}

impl ModuleInstantiateInfo {
    pub fn into_wasm_msg(self, dao: Addr) -> WasmMsg {
        WasmMsg::Instantiate {
            admin: self.admin.map(|admin| match admin {
                Admin::Address { addr } => addr,
                Admin::CoreModule {} => dao.into_string(),
            }),
            code_id: self.code_id,
            msg: self.msg,
            funds: vec![],
            label: self.label,
        }
    }
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{to_binary, Addr, WasmMsg};

    use crate::{Admin, ModuleInstantiateInfo};

    #[test]
    fn test_module_instantiate_admin_none() {
        let no_admin = ModuleInstantiateInfo {
            code_id: 42,
            msg: to_binary("foo").unwrap(),
            admin: None,
            label: "bar".to_string(),
        };
        assert_eq!(
            no_admin.into_wasm_msg(Addr::unchecked("ekez")),
            WasmMsg::Instantiate {
                admin: None,
                code_id: 42,
                msg: to_binary("foo").unwrap(),
                funds: vec![],
                label: "bar".to_string()
            }
        )
    }

    #[test]
    fn test_module_instantiate_admin_addr() {
        let no_admin = ModuleInstantiateInfo {
            code_id: 42,
            msg: to_binary("foo").unwrap(),
            admin: Some(Admin::Address {
                addr: "core".to_string(),
            }),
            label: "bar".to_string(),
        };
        assert_eq!(
            no_admin.into_wasm_msg(Addr::unchecked("ekez")),
            WasmMsg::Instantiate {
                admin: Some("core".to_string()),
                code_id: 42,
                msg: to_binary("foo").unwrap(),
                funds: vec![],
                label: "bar".to_string()
            }
        )
    }

    #[test]
    fn test_module_instantiate_instantiator_addr() {
        let no_admin = ModuleInstantiateInfo {
            code_id: 42,
            msg: to_binary("foo").unwrap(),
            admin: Some(Admin::CoreModule {}),
            label: "bar".to_string(),
        };
        assert_eq!(
            no_admin.into_wasm_msg(Addr::unchecked("ekez")),
            WasmMsg::Instantiate {
                admin: Some("ekez".to_string()),
                code_id: 42,
                msg: to_binary("foo").unwrap(),
                funds: vec![],
                label: "bar".to_string()
            }
        )
    }
}
