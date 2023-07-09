use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Binary, CosmosMsg, WasmMsg};

/// Top level config type for core module.
#[cw_serde]
pub struct Config {
    /// The name of the contract.
    pub name: String,
    /// A description of the contract.
    pub description: String,
    /// An optional image URL for displaying alongside the contract.
    pub image_url: Option<String>,
    /// If true the contract will automatically add received cw20
    /// tokens to its treasury.
    pub automatically_add_cw20s: bool,
    /// If true the contract will automatically add received cw721
    /// tokens to its treasury.
    pub automatically_add_cw721s: bool,
    /// The URI for the DAO as defined by the DAOstar standard
    /// <https://daostar.one/EIP>
    pub dao_uri: Option<String>,
}

/// Top level type describing a proposal module.
#[cw_serde]
pub struct ProposalModule {
    /// The address of the proposal module.
    pub address: Addr,
    /// The URL prefix of this proposal module as derived from the module ID.
    /// Prefixes are mapped to letters, e.g. 0 is 'A', and 26 is 'AA'.
    pub prefix: String,
    /// The status of the proposal module, e.g. 'Enabled' or 'Disabled.'
    pub status: ProposalModuleStatus,
}

/// The status of a proposal module.
#[cw_serde]
pub enum ProposalModuleStatus {
    Enabled,
    Disabled,
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

/// Callbacks to be executed when a module is instantiated
#[cw_serde]
pub struct ModuleInstantiateCallback {
    pub msgs: Vec<CosmosMsg>,
}

#[cfg(test)]
mod tests {
    use super::*;

    use cosmwasm_std::{to_binary, Addr, WasmMsg};

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
