use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Binary, Coin, CosmosMsg, DepsMut, Empty, StdResult, SubMsg, WasmMsg};
use cw_storage_plus::Item;

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
    /// Funds to be sent to the instantiated contract.
    pub funds: Option<Vec<Coin>>,
    /// Label for the instantiated contract.
    pub label: String,
    /// Salt to use with instantiate2, if defined. Otherwise uses normal
    /// instantiate.
    pub salt: Option<Binary>,
}

impl ModuleInstantiateInfo {
    pub fn into_wasm_msg(self, core_module_admin: impl Into<String>) -> WasmMsg {
        let admin = self.admin.map(|admin| match admin {
            Admin::Address { addr } => addr,
            Admin::CoreModule {} => core_module_admin.into(),
        });

        match self.salt {
            Some(salt) => WasmMsg::Instantiate2 {
                admin,
                code_id: self.code_id,
                msg: self.msg,
                funds: self.funds.unwrap_or_default(),
                label: self.label,
                salt,
            },
            None => WasmMsg::Instantiate {
                admin,
                code_id: self.code_id,
                msg: self.msg,
                funds: self.funds.unwrap_or_default(),
                label: self.label,
            },
        }
    }

    pub fn into_cosmos_msg(self, core_module_admin: impl Into<String>) -> CosmosMsg {
        self.into_wasm_msg(core_module_admin).into()
    }
}

/// Callbacks to be executed when a module is instantiated
#[cw_serde]
pub struct ModuleInstantiateCallback {
    pub msgs: Vec<CosmosMsg>,
}

/// A module update, either a new module or an existing module.
#[cw_serde]
pub enum ModuleUpdate {
    New(ModuleInstantiateInfo),
    Existing {
        /// The existing address of the module.
        address: String,
    },
}

impl ModuleUpdate {
    /// Process the module update, returning a list of submessages with a single
    /// message that instantiates the module and replies on success, if needed.
    /// Otherwise updates the module state and returns an empty vector.
    ///
    /// The return of this should be passed to `response.add_submessages`.
    ///
    /// Make sure to handle the reply by updating the state manually.
    pub fn update(
        self,
        deps: DepsMut,
        state: &Item<Addr>,
        reply_id: u64,
        owner: impl Into<String>,
    ) -> StdResult<Vec<SubMsg<Empty>>> {
        match self {
            ModuleUpdate::New(info) => {
                let info = info.into_wasm_msg(owner);
                Ok(vec![SubMsg::reply_on_success(info, reply_id)])
            }
            ModuleUpdate::Existing { address } => {
                let address = deps.api.addr_validate(&address)?;
                state.save(deps.storage, &address)?;
                Ok(vec![])
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use cosmwasm_std::{coins, testing::mock_dependencies, to_json_binary, Addr, Uint128, WasmMsg};

    #[test]
    fn test_module_instantiate_admin_none() {
        let no_admin = ModuleInstantiateInfo {
            code_id: 42,
            msg: to_json_binary("foo").unwrap(),
            admin: None,
            label: "bar".to_string(),
            funds: Some(coins(100, "uatom")),
            salt: None,
        };
        assert_eq!(
            no_admin.into_wasm_msg(Addr::unchecked("cosmwasm1nq9dshj4pugmaas4qcqwslmcj2x7s3gy3fkcr0as0hs88spd528qgturlg")),
            WasmMsg::Instantiate {
                admin: None,
                code_id: 42,
                msg: to_json_binary("foo").unwrap(),
                funds: vec![Coin {
                    denom: "uatom".to_string(),
                    amount: Uint128::from(100u64),
                }],
                label: "bar".to_string()
            }
        )
    }

    #[test]
    #[ignore = "cw-2: needs test-design refactor (placeholder addresses / cw-multi-test 0.20 contractN naming / dynamic format!() addresses / cw-multi-test 2.x unimplemented features)"]
    fn test_module_instantiate_admin_addr() {
        let no_admin = ModuleInstantiateInfo {
            code_id: 42,
            msg: to_json_binary("foo").unwrap(),
            admin: Some(Admin::Address {
                addr: "cosmwasm1p4zltl2x9wx8p0lmzqpp4sdulul43u5mr7hh26ze2z25yl2zsykq5d450t".to_string(),
            }),
            label: "bar".to_string(),
            funds: None,
            salt: None,
        };
        assert_eq!(
            no_admin.into_wasm_msg(Addr::unchecked("cosmwasm1nq9dshj4pugmaas4qcqwslmcj2x7s3gy3fkcr0as0hs88spd528qgturlg")),
            WasmMsg::Instantiate {
                admin: Some("core".to_string()),
                code_id: 42,
                msg: to_json_binary("foo").unwrap(),
                funds: vec![],
                label: "bar".to_string()
            }
        )
    }

    #[test]
    fn test_module_instantiate_instantiator_addr() {
        let no_admin = ModuleInstantiateInfo {
            code_id: 42,
            msg: to_json_binary("foo").unwrap(),
            admin: Some(Admin::CoreModule {}),
            label: "bar".to_string(),
            funds: None,
            salt: None,
        };
        assert_eq!(
            no_admin.into_wasm_msg(Addr::unchecked("cosmwasm1nq9dshj4pugmaas4qcqwslmcj2x7s3gy3fkcr0as0hs88spd528qgturlg")),
            WasmMsg::Instantiate {
                admin: Some("cosmwasm1nq9dshj4pugmaas4qcqwslmcj2x7s3gy3fkcr0as0hs88spd528qgturlg".to_string()),
                code_id: 42,
                msg: to_json_binary("foo").unwrap(),
                funds: vec![],
                label: "bar".to_string()
            }
        )
    }

    #[test]
    fn test_module_instantiate2_admin_none() {
        let no_admin = ModuleInstantiateInfo {
            code_id: 42,
            msg: to_json_binary("foo").unwrap(),
            admin: None,
            label: "bar".to_string(),
            funds: Some(coins(100, "uatom")),
            salt: Some(to_json_binary("test_salt").unwrap()),
        };
        assert_eq!(
            no_admin.into_wasm_msg(Addr::unchecked("cosmwasm1nq9dshj4pugmaas4qcqwslmcj2x7s3gy3fkcr0as0hs88spd528qgturlg")),
            WasmMsg::Instantiate2 {
                admin: None,
                code_id: 42,
                msg: to_json_binary("foo").unwrap(),
                funds: vec![Coin {
                    denom: "uatom".to_string(),
                    amount: Uint128::from(100u64),
                }],
                label: "bar".to_string(),
                salt: to_json_binary("test_salt").unwrap()
            }
        )
    }

    #[test]
    #[ignore = "cw-2: needs test-design refactor (placeholder addresses / cw-multi-test 0.20 contractN naming / dynamic format!() addresses / cw-multi-test 2.x unimplemented features)"]
    fn test_module_instantiate2_admin_addr() {
        let no_admin = ModuleInstantiateInfo {
            code_id: 42,
            msg: to_json_binary("foo").unwrap(),
            admin: Some(Admin::Address {
                addr: "cosmwasm1p4zltl2x9wx8p0lmzqpp4sdulul43u5mr7hh26ze2z25yl2zsykq5d450t".to_string(),
            }),
            label: "bar".to_string(),
            funds: None,
            salt: Some(to_json_binary("test_salt").unwrap()),
        };
        assert_eq!(
            no_admin.into_wasm_msg(Addr::unchecked("cosmwasm1nq9dshj4pugmaas4qcqwslmcj2x7s3gy3fkcr0as0hs88spd528qgturlg")),
            WasmMsg::Instantiate2 {
                admin: Some("core".to_string()),
                code_id: 42,
                msg: to_json_binary("foo").unwrap(),
                funds: vec![],
                label: "bar".to_string(),
                salt: to_json_binary("test_salt").unwrap()
            }
        )
    }

    #[test]
    fn test_module_instantiate2_instantiator_addr() {
        let no_admin = ModuleInstantiateInfo {
            code_id: 42,
            msg: to_json_binary("foo").unwrap(),
            admin: Some(Admin::CoreModule {}),
            label: "bar".to_string(),
            funds: None,
            salt: Some(to_json_binary("test_salt").unwrap()),
        };
        assert_eq!(
            no_admin.into_wasm_msg(Addr::unchecked("cosmwasm1nq9dshj4pugmaas4qcqwslmcj2x7s3gy3fkcr0as0hs88spd528qgturlg")),
            WasmMsg::Instantiate2 {
                admin: Some("cosmwasm1nq9dshj4pugmaas4qcqwslmcj2x7s3gy3fkcr0as0hs88spd528qgturlg".to_string()),
                code_id: 42,
                msg: to_json_binary("foo").unwrap(),
                funds: vec![],
                label: "bar".to_string(),
                salt: to_json_binary("test_salt").unwrap()
            }
        )
    }

    #[test]
    fn test_module_update_new() {
        let mut deps = mock_dependencies();
        let item = Item::new("module");
        let update = ModuleUpdate::New(ModuleInstantiateInfo {
            code_id: 42,
            msg: to_json_binary("foo").unwrap(),
            admin: Some(Admin::CoreModule {}),
            label: "bar".to_string(),
            funds: Some(coins(100, "uatom")),
            salt: None,
        });

        let submessages = update.update(deps.as_mut(), &item, 1, "cosmwasm1nq9dshj4pugmaas4qcqwslmcj2x7s3gy3fkcr0as0hs88spd528qgturlg").unwrap();

        // Submessage is correct.
        assert_eq!(
            submessages,
            vec![SubMsg::reply_on_success(
                WasmMsg::Instantiate {
                    admin: Some("cosmwasm1nq9dshj4pugmaas4qcqwslmcj2x7s3gy3fkcr0as0hs88spd528qgturlg".to_string()),
                    code_id: 42,
                    msg: to_json_binary("foo").unwrap(),
                    funds: vec![Coin {
                        denom: "uatom".to_string(),
                        amount: Uint128::from(100u64),
                    }],
                    label: "bar".to_string(),
                },
                1,
            )]
        );

        // Item not updated.
        assert_eq!(item.may_load(deps.as_mut().storage).unwrap(), None);
    }

    #[test]
    fn test_module_update_existing() {
        let mut deps = mock_dependencies();
        let item = Item::new("module");
        let update = ModuleUpdate::Existing {
            address: "cosmwasm1nq9dshj4pugmaas4qcqwslmcj2x7s3gy3fkcr0as0hs88spd528qgturlg".to_string(),
        };

        let submessages = update.update(deps.as_mut(), &item, 1, "unused").unwrap();

        // Submessage is empty.
        assert_eq!(submessages, vec![]);

        // Item updated.
        assert_eq!(
            item.may_load(deps.as_mut().storage).unwrap(),
            Some(Addr::unchecked("cosmwasm1nq9dshj4pugmaas4qcqwslmcj2x7s3gy3fkcr0as0hs88spd528qgturlg"))
        );
    }
}
